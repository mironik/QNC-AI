//! The card step of a story (docs/01-wizard-plan.md, step 2; user 2026-11-11 choice B): the
//! story's temporary work place (`qnc-card-workspace`) is made in the background when its first
//! card is in, the story's cards are written into its helper project (`qnc-wizard-db`: the database
//! is the only link to Ingest), and the wizard's Ingest opens in it as its own program and goes to
//! those cards, also to a card put in later. It opens again when the user goes on with the step.
//! When the story no longer has a card and Ingest is closed, the work place is deleted. It never
//! waits: the tray asks it on every turn.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver};

use qnc_card_workspace::Workspace;

/// What happened since the last turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The work place is ready and Ingest opened in it.
    Opened,
    /// The work place could not be made.
    WorkspaceFailed(String),
    /// Ingest did not start.
    IngestFailed(String),
    /// The cards of the story were not written into the work place.
    CardsFailed(String),
    /// The work place is deleted (the story has no card and Ingest is closed).
    Removed,
    /// The work place could not be deleted yet (something still has it open); tried again later.
    RemoveFailed(String),
}

pub struct CardPreview {
    /// The QNC root a work place is made from.
    root: PathBuf,
    /// The folder of QNC's programs (Ingest and the programs it starts).
    programs: PathBuf,
    making: Option<Receiver<Result<Workspace, String>>>,
    workspace: Option<Workspace>,
    ingest: Option<Child>,
    /// Ingest opens as soon as the work place is ready.
    open_when_ready: bool,
    /// The cards of the story (volume serials) and the ones written into the work place.
    cards: Vec<String>,
    written: Option<Vec<String>>,
    writing: Option<(Vec<String>, Receiver<Result<(), String>>)>,
}

impl CardPreview {
    pub fn new(root: PathBuf, programs: PathBuf) -> Self {
        Self { root, programs, making: None, workspace: None, ingest: None, open_when_ready: false, cards: Vec::new(), written: None, writing: None }
    }

    /// The cards of the story that are in, by volume serial (call it every turn; a change is written).
    pub fn set_cards(&mut self, cards: Vec<String>) {
        self.cards = cards;
    }

    /// The story has cards: its work place is made (when there is none) and Ingest opens in it.
    pub fn open(&mut self) {
        self.open_when_ready = true;
        if self.workspace.is_none() && self.making.is_none() {
            let (send, receive) = mpsc::channel();
            let root = self.root.clone();
            std::thread::spawn(move || {
                let base = qnc_card_workspace::base();
                let _ = send.send(Workspace::make(&root, &base));
            });
            self.making = Some(receive);
        }
    }

    /// Whether Ingest runs now.
    pub fn ingest_open(&mut self) -> bool {
        match &mut self.ingest {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                _ => {
                    self.ingest = None;
                    false
                }
            },
            None => false,
        }
    }

    /// The work place of the story, when it is ready.
    pub fn workspace(&self) -> Option<&Workspace> {
        self.workspace.as_ref()
    }

    /// One turn: what the background did; `story_has_cards` says whether the work place is still needed.
    pub fn poll(&mut self, story_has_cards: bool) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(made) = self.making.as_ref().and_then(|making| making.try_recv().ok()) {
            self.making = None;
            match made {
                Ok(workspace) => self.workspace = Some(workspace),
                Err(error) => {
                    self.open_when_ready = false;
                    events.push(Event::WorkspaceFailed(error));
                }
            }
        }
        self.write_cards(&mut events);
        // Ingest opens once the cards are in the work place (it reads them from there).
        if self.open_when_ready && self.workspace.is_some() && self.written.as_ref() == Some(&self.cards) {
            self.open_when_ready = false;
            if !self.ingest_open() {
                match self.start_ingest() {
                    Ok(child) => {
                        self.ingest = Some(child);
                        events.push(Event::Opened);
                    }
                    Err(error) => events.push(Event::IngestFailed(error)),
                }
            }
        }
        if !story_has_cards && self.making.is_none() && self.writing.is_none() && !self.ingest_open() {
            if let Some(workspace) = self.workspace.take() {
                match workspace.remove() {
                    Ok(()) => {
                        self.written = None;
                        events.push(Event::Removed);
                    }
                    Err(error) => {
                        events.push(Event::RemoveFailed(error));
                        self.workspace = Some(workspace);
                    }
                }
            }
        }
        events
    }

    /// The story's cards into the work place's helper project, on a thread, when they changed.
    fn write_cards(&mut self, events: &mut Vec<Event>) {
        if let Some((cards, done)) = self.writing.take() {
            match done.try_recv() {
                Ok(Ok(())) => self.written = Some(cards),
                Ok(Err(error)) => {
                    // Not tried again with the same cards: the reason is told once.
                    events.push(Event::CardsFailed(error));
                    self.written = Some(cards);
                }
                Err(mpsc::TryRecvError::Empty) => {
                    self.writing = Some((cards, done));
                    return;
                }
                Err(mpsc::TryRecvError::Disconnected) => {}
            }
        }
        let Some(workspace) = &self.workspace else { return };
        if self.written.as_ref() == Some(&self.cards) {
            return;
        }
        let (send, receive) = mpsc::channel();
        let (root, cards) = (workspace.root.clone(), self.cards.clone());
        std::thread::spawn(move || {
            let written = qnc_wizard_db::project_target(&root).and_then(|target| qnc_wizard_db::set_cards(&target, cards));
            let _ = send.send(written);
        });
        self.writing = Some((self.cards.clone(), receive));
    }

    fn start_ingest(&self) -> Result<Child, String> {
        let workspace = self.workspace.as_ref().ok_or("no work place")?;
        // The wizard's Ingest (QNC's Ingest that goes to the cards by itself), beside QNC's programs it starts.
        let program = program(&self.programs, "qnc-ai-ingest");
        let mut command = Command::new(&program);
        command.current_dir(&workspace.root).env("QNC_ROOT", &workspace.root);
        // Ingest and the console programs it starts (its worker, decoders, player) share one console
        // that is not shown: without it each of them flashed a console window of its own.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        command.spawn().map_err(|error| format!("{}: {error}", program.display()))
    }
}

/// A program of QNC in `folder`.
pub fn program(folder: &Path, name: &str) -> PathBuf {
    folder.join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

/// The folder of QNC's programs: `QNC_PROGRAMS` when it is set, else the folder of this program
/// (a package puts the wizard beside QNC's programs).
pub fn programs_folder() -> PathBuf {
    std::env::var_os("QNC_PROGRAMS")
        .map(PathBuf::from)
        .or_else(|| std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)))
        .unwrap_or_default()
}
