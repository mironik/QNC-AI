//! Which steps of the wizard are done in a project: the table `wizard_steps` of the project
//! database (docs/01-wizard-plan.md, section 3: the wizard's state is in the database, so
//! closing and opening again goes on where it stopped). A table module of QNC's project
//! database intermediary (`qnc-db-broker`): it owns only its table, is written only through
//! that intermediary and knows no form. QNC does not read it.

mod store;

use std::collections::BTreeSet;
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use qnc_db_broker::{Access, Pending, ProjectDbTarget, ProjectDbWriter, TableModuleFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MODULE_ID: &str = "qnc-ai.module.wizard-db";

pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    /// The steps that are done.
    Read,
    /// A step is done.
    MarkDone { step: String },
    /// A step is open again (the steps after it stay as they are; the wizard's rule keeps them locked).
    Reopen { step: String },
    /// The cards of the story, by volume serial, in their order (they replace the ones before): the
    /// tray writes them, the wizard's Ingest reads them (the database is the only link between them).
    SetCards { serials: Vec<String> },
    /// The cards of the story.
    ReadCards,
}

impl Operation {
    pub fn is_write(&self) -> bool {
        !matches!(self, Self::Read | Self::ReadCards)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Data {
    Done(BTreeSet<String>),
    Cards(Vec<String>),
    Changed,
}

/// The project database of the QNC root `root`'s active project (the story's project, or a work
/// place's helper project), as QNC's own blocks find it.
pub fn project_target(root: &std::path::Path) -> Result<ProjectDbTarget> {
    let reader = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|error| error.to_string())?;
    let snapshot = reader.read().map_err(|error| error.to_string())?;
    ProjectDbTarget::for_project(reader.settings_reader(), &snapshot.settings)
}

/// Writes the cards of the story into the project behind `target` (waits for the write).
pub fn set_cards(target: &ProjectDbTarget, serials: Vec<String>) -> Result<()> {
    let writer = ProjectDbWriter::start(target.clone(), vec![WizardModule::factory()])?;
    writer.call(MODULE_ID, encode(&Operation::SetCards { serials })?).map(|_| ())
}

/// The cards of the story in the project behind `target` (none before the tray wrote any).
pub fn read_cards(target: &ProjectDbTarget) -> Result<Vec<String>> {
    let mut client = target.open(Access::ReadOnly, vec![WizardModule::factory()])?;
    let answer = client.execute(MODULE_ID, encode(&Operation::ReadCards)?)?;
    match serde_json::from_value(answer).map_err(|e| e.to_string())? {
        Data::Cards(cards) => Ok(cards),
        _ => Err(qnc_wizard_text::text("wizard.db.reply")),
    }
}

/// Joins the wizard table to a project database.
pub struct WizardModule;

impl WizardModule {
    pub fn factory() -> Arc<dyn TableModuleFactory> {
        qnc_db_broker::owned_tables::<store::Store>()
    }
}

fn encode(operation: &Operation) -> Result<Value> {
    serde_json::to_value(operation).map_err(|e| e.to_string())
}

/// The steps done in the project behind `target` (none before the wizard wrote any).
pub fn read(target: &ProjectDbTarget) -> Result<BTreeSet<String>> {
    let mut client = target.open(Access::ReadOnly, vec![WizardModule::factory()])?;
    let answer = client.execute(MODULE_ID, encode(&Operation::Read)?)?;
    match serde_json::from_value(answer).map_err(|e| e.to_string())? {
        Data::Done(done) => Ok(done),
        _ => Err(qnc_wizard_text::text("wizard.db.reply")),
    }
}

/// How often the state is read again while the wizard shows a project.
pub const EVERY: Duration = Duration::from_secs(2);

/// The wizard's state of the project one window shows: read in the background and again every
/// `EVERY` (another program may change it), written through the project's serial writer without
/// waiting. The last answer of the database is all it keeps.
#[derive(Default)]
pub struct WizardState {
    project: Option<String>,
    done: Option<BTreeSet<String>>,
    error: Option<String>,
    reading: Option<mpsc::Receiver<Result<BTreeSet<String>>>>,
    read_at: Option<Instant>,
    writer: Option<ProjectDbWriter>,
    pending: Vec<Pending>,
}

impl WizardState {
    pub fn new() -> Self {
        Self::default()
    }

    /// The steps done in the project of `target` (`None` until the first answer), read again in
    /// the background when it is time. Call it every frame; it never waits.
    pub fn poll(&mut self, target: Option<&ProjectDbTarget>) -> Option<&BTreeSet<String>> {
        let Some(target) = target else {
            *self = Self::default();
            return None;
        };
        if self.project.as_deref() != Some(target.uri()) {
            *self = Self { project: Some(target.uri().to_string()), ..Self::default() };
        }
        self.pending.retain(|pending| match pending.try_take() {
            None => true,
            Some(Ok(_)) => false,
            Some(Err(error)) => {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("wizard-db not written: {error}"));
                false
            }
        });
        if let Some(answer) = self.reading.as_ref().and_then(|reading| reading.try_recv().ok()) {
            self.reading = None;
            // A read started before this window's own write could undo it: it counts only with no write on its way.
            if self.pending.is_empty() {
                match answer {
                    Ok(done) => {
                        self.done = Some(done);
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        }
        let due = self.read_at.is_none_or(|at| at.elapsed() >= EVERY);
        if self.reading.is_none() && self.pending.is_empty() && due {
            let (send, receive) = mpsc::channel();
            let owned = target.clone();
            std::thread::spawn(move || {
                let _ = send.send(read(&owned));
            });
            self.reading = Some(receive);
            self.read_at = Some(Instant::now());
        }
        self.done.as_ref()
    }

    /// The steps done as last read (or as this window just wrote them).
    pub fn done(&self) -> Option<&BTreeSet<String>> {
        self.done.as_ref()
    }

    /// Why the state cannot be read, when it cannot.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Whether a write is on its way.
    pub fn writing(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Marks `step` done (or open again) in the project of `target`; shown at once.
    pub fn set(&mut self, target: &ProjectDbTarget, step: &str, done: bool) {
        if !qnc_wizard_steps::is_step(step) {
            self.error = Some(qnc_wizard_text::text("wizard.db.bad_step"));
            return;
        }
        if self.writer.is_none() {
            match ProjectDbWriter::spawn(target.clone(), vec![WizardModule::factory()]) {
                Ok(writer) => self.writer = Some(writer),
                Err(error) => {
                    self.error = Some(error);
                    return;
                }
            }
        }
        let Some(writer) = &self.writer else { return };
        let operation = if done { Operation::MarkDone { step: step.into() } } else { Operation::Reopen { step: step.into() } };
        match encode(&operation).and_then(|payload| writer.submit(MODULE_ID, payload)) {
            Ok(pending) => self.pending.push(pending),
            Err(error) => {
                self.error = Some(error);
                return;
            }
        }
        let shown = self.done.get_or_insert_with(BTreeSet::new);
        if done {
            shown.insert(step.into());
        } else {
            shown.remove(step);
        }
        self.read_at = None;
    }
}

#[cfg(test)]
mod tests;
