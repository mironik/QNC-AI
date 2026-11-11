//! What the wizard shows and what its buttons do. The active project is read from the database
//! of the root (QNC's active project port), again every `EVERY`, in the background; which
//! steps are done is the project database's (`qnc-wizard-db`); where the user may go is the
//! wizard's one rule (`qnc-wizard-steps`). The step the user looks at is a choice of this
//! window, nothing else is kept. It draws nothing.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use qnc_db_broker::ProjectDbTarget;
use qnc_wizard_db::WizardState;
use qnc_wizard_steps::{Progress, Step};

/// How often the active project is read again.
pub const EVERY: Duration = Duration::from_secs(2);

/// What a button of the wizard asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Back,
    Next,
    Open(usize),
    /// The shown step is done (until its own condition is made, docs/01-wizard-plan.md W2-W6).
    MarkDone,
    /// The shown step is open again.
    Reopen,
}

/// The project the wizard works on, as last read.
#[derive(Debug, Clone)]
enum Project {
    Reading,
    None,
    Error(String),
    Open { name: String, target: ProjectDbTarget },
}

/// One step as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRow {
    pub name: String,
    pub done: bool,
    pub may_open: bool,
    pub shown: bool,
}

/// Everything the window draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub project: Option<String>,
    /// Why there is nothing to work on, when there is not.
    pub message: Option<String>,
    pub steps: Vec<StepRow>,
    pub shown: usize,
    pub shown_name: String,
    pub done_when: String,
    pub shown_done: bool,
    pub may_back: bool,
    pub may_next: bool,
    pub may_mark: bool,
    pub saving: bool,
}

pub struct WizardApplication {
    root: PathBuf,
    steps: &'static [Step],
    project: Project,
    reading: Option<mpsc::Receiver<Project>>,
    read_at: Option<Instant>,
    state: WizardState,
    /// The step the user looks at; `None` follows where the user is.
    shown: Option<usize>,
}

impl WizardApplication {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            steps: qnc_wizard_steps::steps(),
            project: Project::Reading,
            reading: None,
            read_at: None,
            state: WizardState::new(),
            shown: None,
        }
    }

    fn target(&self) -> Option<&ProjectDbTarget> {
        match &self.project {
            Project::Open { target, .. } => Some(target),
            _ => None,
        }
    }

    /// Takes what the background read and starts the next read when it is time. Never waits.
    pub fn poll(&mut self) {
        if let Some(project) = self.reading.as_ref().and_then(|reading| reading.try_recv().ok()) {
            self.reading = None;
            let changed = match (&self.project, &project) {
                (Project::Open { target: was, .. }, Project::Open { target: now, .. }) => was.uri() != now.uri(),
                _ => true,
            };
            if changed {
                self.shown = None;
            }
            self.project = project;
        }
        if self.reading.is_none() && self.read_at.is_none_or(|at| at.elapsed() >= EVERY) {
            let (send, receive) = mpsc::channel();
            let root = self.root.clone();
            std::thread::spawn(move || {
                let _ = send.send(read_project(&root));
            });
            self.reading = Some(receive);
            self.read_at = Some(Instant::now());
        }
        let target = self.target().cloned();
        self.state.poll(target.as_ref());
    }

    fn progress(&self, done: &BTreeSet<String>) -> Progress {
        Progress::new(self.steps, done)
    }

    pub fn view(&self) -> View {
        let empty = BTreeSet::new();
        let done = self.state_done().unwrap_or(&empty);
        let progress = self.progress(done);
        let shown = self.shown.unwrap_or_else(|| progress.current()).min(self.steps.len() - 1);
        let message = match &self.project {
            Project::Reading => Some(String::new()),
            Project::None => Some(qnc_wizard_text::text("wizard.no_project")),
            Project::Error(error) => Some(qnc_wizard_text::fill("wizard.project_error", &[("error", error)])),
            Project::Open { .. } => self.state.error().map(|error| qnc_wizard_text::fill("wizard.state_error", &[("error", error)])),
        };
        let open = self.target().is_some() && self.state_done().is_some();
        let step = &self.steps[shown];
        View {
            project: match &self.project {
                Project::Open { name, .. } => Some(name.clone()),
                _ => None,
            },
            message,
            steps: self
                .steps
                .iter()
                .enumerate()
                .map(|(index, step)| StepRow { name: step.name.clone(), done: progress.is_done(index), may_open: open && progress.may_open(index), shown: index == shown })
                .collect(),
            shown,
            shown_name: step.name.clone(),
            done_when: step.done_when.clone(),
            shown_done: progress.is_done(shown),
            may_back: open && progress.back(shown).is_some(),
            may_next: open && progress.next(shown).is_some(),
            may_mark: open,
            saving: self.state.writing(),
        }
    }

    fn state_done(&self) -> Option<&BTreeSet<String>> {
        self.state.done()
    }

    pub fn act(&mut self, intent: Intent) {
        let view = self.view();
        let Some(target) = self.target().cloned() else { return };
        match intent {
            Intent::Back if view.may_back => self.shown = Some(view.shown - 1),
            Intent::Next if view.may_next => self.shown = Some(view.shown + 1),
            Intent::Open(index) if view.steps.get(index).is_some_and(|row| row.may_open) => self.shown = Some(index),
            Intent::MarkDone if view.may_mark => self.state.set(&target, &self.steps[view.shown].code, true),
            Intent::Reopen if view.may_mark => self.state.set(&target, &self.steps[view.shown].code, false),
            _ => {}
        }
    }

    /// Whether something is on its way (the window draws again soon).
    pub fn busy(&self) -> bool {
        self.reading.is_some() || self.state.writing()
    }
}

/// The active project of `root`, read once.
fn read_project(root: &std::path::Path) -> Project {
    let reader = match qnc_active_project_read::ActiveProjectReader::from_root(root) {
        Ok(reader) => reader,
        Err(error) if error.code == "no_active_project" => return Project::None,
        Err(error) => return Project::Error(error.to_string()),
    };
    match reader.read() {
        Ok(snapshot) => match ProjectDbTarget::for_project(reader.settings_reader(), &snapshot.settings) {
            Ok(target) => Project::Open { name: snapshot.settings.project_name.clone(), target },
            Err(error) => Project::Error(error),
        },
        Err(error) if error.code == "no_active_project" => Project::None,
        Err(error) => Project::Error(error.to_string()),
    }
}
