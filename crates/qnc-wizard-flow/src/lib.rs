//! Where the story is on its way from the card to the export (docs/01-wizard-plan.md). The
//! wizard has no window of its own (user 2026-11-11: "ne zelim formu vizarda, vizard moze biti
//! sastavni dio traya"): the tray program asks this block where the story is, shows it in its
//! menu and tooltip and opens the form of that step. Before a story is made the step is the card
//! (only in the memory of the tray program); after it, the done steps are the project database's
//! (`qnc-wizard-db`), read again in the background. It draws nothing and opens nothing.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use qnc_db_broker::ProjectDbTarget;
use qnc_wizard_db::WizardState;
use qnc_wizard_steps::{Progress, Step};

/// The first step: the card and its shots (before any project).
pub const CARD_STEP: &str = "card";

/// How often the active project is read again.
pub const EVERY: Duration = Duration::from_secs(2);

/// The card a story starts with: its disk and the cameras the catalog found on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub disk: PathBuf,
    pub cameras: Vec<String>,
}

/// The project the story is in, as last read.
#[derive(Debug, Clone)]
enum Project {
    Reading,
    None,
    Error(String),
    Open { name: String, target: ProjectDbTarget },
}

/// Where the story is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The step the story is at (0 based) and how many steps there are.
    pub step: usize,
    pub steps: usize,
    pub code: String,
    pub name: String,
    pub done_when: String,
    /// The story's project, when there is one.
    pub project: Option<String>,
    /// The card the story starts with, when it starts with one.
    pub card: Option<String>,
    /// Why the story cannot go on, when it cannot.
    pub problem: Option<String>,
}

impl Status {
    /// One line for the tray: "3/11 Import: <project or card>".
    pub fn line(&self) -> String {
        let place = self.project.clone().or_else(|| self.card.clone()).unwrap_or_default();
        let line = format!("{}/{} {}", self.step + 1, self.steps, self.name);
        if place.is_empty() {
            line
        } else {
            format!("{line}: {place}")
        }
    }
}

pub struct Flow {
    root: PathBuf,
    steps: &'static [Step],
    card: Option<Card>,
    project: Project,
    reading: Option<mpsc::Receiver<Project>>,
    read_at: Option<Instant>,
    state: WizardState,
}

impl Flow {
    pub fn new(root: PathBuf) -> Self {
        Self { root, steps: qnc_wizard_steps::steps(), card: None, project: Project::Reading, reading: None, read_at: None, state: WizardState::new() }
    }

    /// A card was put in: a new story starts with it (its project is made in the step after).
    pub fn start_with(&mut self, card: Card) {
        self.card = Some(card);
    }

    /// The card the story starts with, while it has no project yet.
    pub fn card(&self) -> Option<&Card> {
        self.card.as_ref()
    }

    fn target(&self) -> Option<&ProjectDbTarget> {
        match &self.project {
            Project::Open { target, .. } => Some(target),
            _ => None,
        }
    }

    /// Takes what the background read and starts the next read when it is time. Never waits.
    /// With a card and no story made yet, no project is read: the story is a new one.
    pub fn poll(&mut self) {
        if let Some(project) = self.reading.as_ref().and_then(|reading| reading.try_recv().ok()) {
            self.reading = None;
            self.project = project;
        }
        if self.card.is_none() && self.reading.is_none() && self.read_at.is_none_or(|at| at.elapsed() >= EVERY) {
            let (send, receive) = mpsc::channel();
            let root = self.root.clone();
            std::thread::spawn(move || {
                let _ = send.send(read_project(&root));
            });
            self.reading = Some(receive);
            self.read_at = Some(Instant::now());
        }
        let target = if self.card.is_some() { None } else { self.target().cloned() };
        self.state.poll(target.as_ref());
    }

    /// Where the story is.
    pub fn status(&self) -> Status {
        let open = self.card.is_none() && self.target().is_some();
        let mut done: BTreeSet<String> = if open { self.state.done().cloned().unwrap_or_default() } else { BTreeSet::new() };
        // The card step is done once the story has a project: a story is made only from chosen shots.
        if open {
            done.insert(CARD_STEP.into());
        }
        let step = Progress::new(self.steps, &done).current();
        let problem = match (&self.card, &self.project) {
            (Some(_), _) | (None, Project::Reading) | (None, Project::None) => None,
            (None, Project::Error(error)) => Some(qnc_wizard_text::fill("wizard.project_error", &[("error", error)])),
            (None, Project::Open { .. }) => self.state.error().map(|error| qnc_wizard_text::fill("wizard.state_error", &[("error", error)])),
        };
        Status {
            step,
            steps: self.steps.len(),
            code: self.steps[step].code.clone(),
            name: self.steps[step].name.clone(),
            done_when: self.steps[step].done_when.clone(),
            project: match (&self.card, &self.project) {
                (None, Project::Open { name, .. }) => Some(name.clone()),
                _ => None,
            },
            card: self.card.as_ref().map(|card| qnc_wizard_text::fill("wizard.card", &[("disk", &card.disk.display().to_string()), ("camera", &card.cameras.join(", "))])),
            problem,
        }
    }

    /// Whether something is on its way.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_story_put_in_with_a_card_is_at_the_card_step_and_reads_no_project() {
        let mut flow = Flow::new(PathBuf::from("no-such-root"));
        flow.start_with(Card { disk: PathBuf::from("D:\\"), cameras: vec!["Sony".into()] });
        flow.poll();
        assert!(flow.reading.is_none(), "a new story reads no project");
        let status = flow.status();
        assert_eq!((status.step, status.code.as_str()), (0, CARD_STEP));
        assert_eq!(status.project, None);
        assert_eq!(status.line(), "1/11 Card and shots: card D:\\ (Sony)");
    }
}
