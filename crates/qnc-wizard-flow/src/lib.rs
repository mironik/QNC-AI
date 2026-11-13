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

/// The card a story starts with: its disk, its volume serial and label, and the cameras the catalog
/// found on it. A card taken out without ejecting it stays in the story as `out` until it is put
/// back (user 2026-11-11: "vratite karticu ... koristi se").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub disk: PathBuf,
    pub serial: String,
    pub label: Option<String>,
    pub cameras: Vec<String>,
    pub out: bool,
}

impl Card {
    /// The card's name for a person: its label and disk ("XAVC (D:\)"), the disk without a label.
    pub fn name(&self) -> String {
        match &self.label {
            Some(label) if !label.trim().is_empty() => format!("{label} ({})", self.disk.display()),
            _ => self.disk.display().to_string(),
        }
    }

    /// The card's name with its camera ("XAVC (G:\) · Sony ILME-FX3A"): a disk and a number alone
    /// say little (user 2026-11-13).
    pub fn name_with_camera(&self) -> String {
        match self.cameras.is_empty() {
            true => self.name(),
            false => format!("{} · {}", self.name(), self.cameras.join(", ")),
        }
    }
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
    /// The cards the story starts with (a shoot is often on more than one), one line each.
    pub cards: Vec<String>,
    /// Which of them are taken out (to be put back).
    pub cards_out: Vec<bool>,
    /// Why the story cannot go on, when it cannot.
    pub problem: Option<String>,
}

impl Status {
    /// One line for the tray: "3/11 Import: <project>".
    pub fn line(&self) -> String {
        let line = format!("{}/{} {}", self.step + 1, self.steps, self.name);
        match &self.project {
            Some(project) => format!("{line}: {project}"),
            None => line,
        }
    }
}

pub struct Flow {
    root: PathBuf,
    steps: &'static [Step],
    cards: Vec<Card>,
    project: Project,
    reading: Option<mpsc::Receiver<Project>>,
    read_at: Option<Instant>,
    state: WizardState,
}

impl Flow {
    pub fn new(root: PathBuf) -> Self {
        Self { root, steps: qnc_wizard_steps::steps(), cards: Vec::new(), project: Project::Reading, reading: None, read_at: None, state: WizardState::new() }
    }

    /// A card was put in: a new story starts with it, or it joins the cards of the story that has
    /// no project yet (a shoot on two cards). A card of the story put back (the same volume serial,
    /// whatever its letter now) is in again: `true`.
    pub fn add_card(&mut self, card: Card) -> bool {
        if let Some(known) = self.cards.iter_mut().find(|known| known.serial == card.serial) {
            let back = known.out;
            *known = Card { out: false, ..card };
            return back;
        }
        self.cards.retain(|known| known.disk != card.disk);
        self.cards.push(Card { out: false, ..card });
        false
    }

    /// A card of the story was taken out without ejecting it: it stays in the story as out, to be
    /// put back. The card, when it is one of the story's and was in.
    pub fn card_out(&mut self, serial: &str) -> Option<Card> {
        let card = self.cards.iter_mut().find(|known| known.serial == serial && !known.out)?;
        card.out = true;
        Some(card.clone())
    }

    /// A card was ejected on purpose: it leaves the story.
    pub fn remove_card(&mut self, serial: &str) -> bool {
        let before = self.cards.len();
        self.cards.retain(|known| known.serial != serial);
        self.cards.len() != before
    }

    /// The cards the story starts with, while it has no project yet.
    pub fn cards(&self) -> &[Card] {
        &self.cards
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
        if self.cards.is_empty() && self.reading.is_none() && self.read_at.is_none_or(|at| at.elapsed() >= EVERY) {
            let (send, receive) = mpsc::channel();
            let root = self.root.clone();
            std::thread::spawn(move || {
                let _ = send.send(read_project(&root));
            });
            self.reading = Some(receive);
            self.read_at = Some(Instant::now());
        }
        let target = if self.cards.is_empty() { self.target().cloned() } else { None };
        self.state.poll(target.as_ref());
    }

    /// Where the story is.
    pub fn status(&self) -> Status {
        let new_story = !self.cards.is_empty();
        let open = !new_story && self.target().is_some();
        let mut done: BTreeSet<String> = if open { self.state.done().cloned().unwrap_or_default() } else { BTreeSet::new() };
        // The card step is done once the story has a project: a story is made only from chosen shots.
        if open {
            done.insert(CARD_STEP.into());
        }
        let step = Progress::new(self.steps, &done).current();
        let problem = match (new_story, &self.project) {
            (true, _) | (false, Project::Reading) | (false, Project::None) => None,
            (false, Project::Error(error)) => Some(qnc_wizard_text::fill("wizard.project_error", &[("error", error)])),
            (false, Project::Open { .. }) => self.state.error().map(|error| qnc_wizard_text::fill("wizard.state_error", &[("error", error)])),
        };
        Status {
            step,
            steps: self.steps.len(),
            code: self.steps[step].code.clone(),
            name: self.steps[step].name.clone(),
            done_when: self.steps[step].done_when.clone(),
            project: match (new_story, &self.project) {
                (false, Project::Open { name, .. }) => Some(name.clone()),
                _ => None,
            },
            cards: self
                .cards
                .iter()
                .map(|card| {
                    let code = if card.out { "wizard.card_taken_out" } else { "wizard.card" };
                    qnc_wizard_text::fill(code, &[("card", &card.name()), ("camera", &card.cameras.join(", "))])
                })
                .collect(),
            cards_out: self.cards.iter().map(|card| card.out).collect(),
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

    fn card(disk: &str, serial: &str, label: Option<&str>, camera: &str) -> Card {
        Card { disk: PathBuf::from(disk), serial: serial.into(), label: label.map(Into::into), cameras: vec![camera.into()], out: false }
    }

    #[test]
    fn a_story_put_in_with_a_card_is_at_the_card_step_and_reads_no_project() {
        let mut flow = Flow::new(PathBuf::from("no-such-root"));
        flow.add_card(card("D:\\", "83dbcfbe", Some("XAVC"), "Sony FX6"));
        flow.poll();
        assert!(flow.reading.is_none(), "a new story reads no project");
        let status = flow.status();
        assert_eq!((status.step, status.code.as_str()), (0, CARD_STEP));
        assert_eq!(status.project, None);
        assert_eq!(status.line(), "1/11 Card and shots");
        assert_eq!(status.cards, vec!["card XAVC (D:\\), Sony FX6".to_string()]);
    }

    #[test]
    fn a_card_taken_out_stays_to_be_put_back_and_an_ejected_one_leaves() {
        let mut flow = Flow::new(PathBuf::from("no-such-root"));
        flow.add_card(card("D:\\", "83dbcfbe", None, "Sony FX6"));
        flow.add_card(card("G:\\", "30616164", None, "Sony FX3"));
        assert!(!flow.add_card(card("D:\\", "83dbcfbe", None, "Sony FX6")), "the same card again is no card put back");
        assert_eq!(flow.cards().len(), 2);
        assert_eq!(flow.card_out("83dbcfbe").map(|card| card.name()), Some("D:\\".to_string()));
        assert_eq!(flow.card_out("83dbcfbe"), None, "out once");
        assert_eq!(flow.status().cards[0], "card D:\\, Sony FX6: taken out");
        assert!(flow.add_card(card("E:\\", "83dbcfbe", None, "Sony FX6")), "put back on another letter, it is the same card");
        assert_eq!(flow.cards()[0].disk, PathBuf::from("E:\\"));
        assert!(flow.remove_card("30616164"));
        assert!(!flow.remove_card("ffffffff"), "a card the story has not is nothing");
        assert_eq!(flow.cards().len(), 1);
    }
}
