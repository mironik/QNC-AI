//! The background ingest of the cards (user 2026-11-12: "globalna baza kao ingest na velikim
//! ingest sustavima: korisnik ubaci karticu, tray provjeri poznaje li karticu i odmah radi Select
//! All i ingestira materijal po osnovnim postavkama (link media); ingest proces koji pokreće tray
//! je kopiranje postera, kreiranje filmstripa i grube wave"). It drives the Ingest application
//! without a form, through Ingest's own actions, as the user would: every attached card of a known
//! camera is Selected into the global Ingest base, all its clips are selected and imported by the
//! project's settings (link: nothing is copied); the background worker copies the posters and
//! makes the filmstrips and waves. It is done when every clip has its poster, its filmstrip and at
//! least its rough wave ("puni wave ne čekati"); the worker goes on with the full waves alone.
//! What is done is read from the base once a second on a thread of its own. It draws nothing: a
//! window shows `status()`.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use qnc_ingest_application::{action_ids, IngestApplication, IngestIntent};
pub use qnc_work_progress::{progress_of, resume_all, stop, stop_all, Bar, Progress};
use qnc_work_progress::progress;

mod start_ask;
pub use start_ask::{Ask, StartAsk};

/// How often what is done is read from the base.
const READ: Duration = Duration::from_secs(1);

/// Where the ingest is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for a card (or for the base to load).
    Waiting,
    /// A folder is to be picked in Ingest's browser (a source that is no card).
    Choosing,
    /// A card is Selected: its clips are read into the base.
    Selecting,
    /// The worker copies the posters and makes the filmstrips and rough waves.
    Making,
    Done,
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub progress: Progress,
    /// The cameras of the cards attached.
    pub cameras: Vec<String>,
    /// The cards attached, each with the earlier base that already has it.
    pub cards: Vec<CardLine>,
    /// What Ingest said last (a failure among it).
    pub message: String,
}

/// A card attached: its camera and, when an earlier base already has it, that base (the card is
/// then not imported again; the base Ingest works in links to it) and how many clips it has new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLine {
    pub camera: String,
    pub known_in: Option<String>,
    pub new: usize,
}

pub struct CardIngest {
    root: PathBuf,
    application: IngestApplication,
    attached: qnc_attached_cards::AttachedCards,
    auto: qnc_card_autoselect::AutoSelect,
    cameras: Vec<String>,
    cards: Vec<CardLine>,
    /// The cards whose earlier base the base Ingest works in links to already.
    linked: std::collections::BTreeSet<String>,
    /// A Select ran since the clips were imported last.
    selected: bool,
    /// The import was handed to the worker.
    making: bool,
    progress: Progress,
    read: Receiver<Progress>,
    /// The attached cards were looked at once.
    looked: bool,
    /// A folder the user picks (user 2026-11-13), as the source or beside the cards.
    folder: Option<FolderPick>,
    /// The folder or card the user chose in the Ingest form (user 2026-11-16: "kada se bira kartica
    /// unutar ingest forme, ona bi trebala koristiti proces tray-direktorij ingest"): Selected as
    /// soon as the browser is ready, without being picked again.
    preset: Option<String>,
    /// The cards attached are ingested too (not when the window was started for a folder alone).
    cards_too: bool,
    /// The camera of each card attached, by its volume serial (lower case).
    card_cameras: std::collections::BTreeMap<String, String>,
    /// The one card this window ingests (a base per card, user 2026-11-14); `None`: every card.
    only_card: Option<String>,
}

/// Where the pick of a folder is.
#[derive(Debug, Clone, PartialEq, Eq)]
enum FolderPick {
    /// The browser is shown; the disks are listed once.
    Choosing { listed: bool },
    /// Select of the folder sent; waits until Ingest took it.
    Taking(String),
    /// Select runs.
    Selecting,
    /// Select ended: the rest is as for a card.
    Taken,
}

impl CardIngest {
    /// The source is a folder the user picks in Ingest's browser (`view`, `choose`); the cards
    /// attached are left alone.
    pub fn start_folder(root: &Path) -> Result<Self, String> {
        let mut ingest = Self::start(root)?;
        ingest.folder = Some(FolderPick::Choosing { listed: false });
        ingest.cards_too = false;
        ingest.looked = true;
        Ok(ingest)
    }

    /// The source is the folder (or card) of `uri` the Ingest form chose: as `start_folder`, the
    /// folder Selected once the browser is ready.
    pub fn start_folder_at(root: &Path, uri: &str) -> Result<Self, String> {
        let mut ingest = Self::start_folder(root)?;
        ingest.preset = Some(uri.to_string());
        Ok(ingest)
    }

    /// A folder to pick beside what runs (the tray's "Import from a folder" while a card is
    /// ingested, user 2026-11-13): the browser is shown once the card's Select has ended; what
    /// the worker does goes on.
    pub fn pick_folder(&mut self) {
        if !self.choosing() {
            self.folder = Some(FolderPick::Choosing { listed: false });
        }
    }

    /// Back from the browser without a folder; a window started for a folder alone ends.
    pub fn cancel_folder(&mut self) -> bool {
        self.folder = None;
        self.cards_too
    }

    /// What Ingest shows (its browser while a folder is picked).
    pub fn view(&self) -> &qnc_ingest_application::IngestViewModel {
        self.application.view()
    }

    /// Ingests only the card of volume serial `serial`; the other cards attached have windows of
    /// their own, each in its own base (user 2026-11-14).
    pub fn only_card(mut self, serial: &str) -> Self {
        self.only_card = Some(serial.to_ascii_lowercase());
        self
    }

    /// The camera of each card attached, by its volume serial (lower case).
    pub fn card_cameras(&self) -> &std::collections::BTreeMap<String, String> {
        &self.card_cameras
    }

    /// Whether a folder is still to be picked.
    pub fn choosing(&self) -> bool {
        matches!(self.folder, Some(FolderPick::Choosing { .. }))
    }

    /// What the user did in the browser while a folder is picked: Select of a folder starts its
    /// ingest. Anything else after the pick is not taken.
    pub fn choose(&mut self, intent: IngestIntent) {
        if !self.choosing() {
            return;
        }
        if intent.action_id == action_ids::INGEST_DIR_CONFIRM {
            if let qnc_ingest_application::IngestPayload::LocationUri(uri) = &intent.payload {
                // The folder as the browser shows it is written in the base, for the tray to tell.
                let label = self.application.view().browser.path_label.clone();
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-card-ingest folder {uri} ({label})"));
                let root = self.root.clone();
                std::thread::spawn(move || {
                    if let Err(error) = qnc_ingest_base::add_source(&root, &label) {
                        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-card-ingest folder not written: {error}"));
                    }
                });
                self.folder = Some(FolderPick::Taking(uri.clone()));
            }
        }
        self.application.dispatch(intent);
    }

    /// The folder's Select, step by step: taken, running, ended (then the clips go to the import).
    fn folder_step(&mut self) {
        // The folder the Ingest form chose, once the browser is ready: Selected as if picked.
        let ready = |view: &qnc_ingest_application::IngestViewModel| !view.command_busy && !view.browser.busy && !view.work_settings_loading;
        if matches!(self.folder, Some(FolderPick::Choosing { listed: true })) && ready(self.application.view()) {
            if let Some(uri) = self.preset.take() {
                self.choose(IngestIntent::new(action_ids::INGEST_DIR_CONFIRM, qnc_ingest_application::IngestPayload::LocationUri(uri)));
                return;
            }
        }
        let view = self.application.view();
        let next = match &self.folder {
            // The browser is Ingest's: it is taken once no card is being gone to.
            Some(FolderPick::Choosing { listed: false }) if !self.auto.busy() && !view.command_busy && !view.browser.busy && !view.work_settings_loading => {
                self.application.dispatch(IngestIntent::empty(action_ids::INGEST_SOURCE_KIND_LOCAL));
                Some(FolderPick::Choosing { listed: true })
            }
            Some(FolderPick::Taking(uri)) if view.command_busy || view.selected_source_uri.as_deref() == Some(uri.as_str()) => Some(FolderPick::Selecting),
            Some(FolderPick::Selecting) if !view.command_busy => {
                self.selected = true;
                Some(FolderPick::Taken)
            }
            _ => None,
        };
        if let Some(next) = next {
            self.folder = Some(next);
        }
    }

    pub fn start(root: &Path) -> Result<Self, String> {
        let (send, read) = channel();
        let at = root.to_path_buf();
        std::thread::spawn(move || {
            let mut said = String::new();
            // The base this ingest works in: once the active project is another one (the base
            // closed to be deleted, another opened) the window lets it go (live 2026-11-12: a
            // window still open held the base the tray deleted).
            let mut base: Option<String> = None;
            loop {
                let now = qnc_active_project_read::ActiveProjectReader::from_root(&at)
                    .ok()
                    .and_then(|active| active.read().ok())
                    .map(|snapshot| snapshot.settings.project_id);
                if base.is_none() {
                    base = now.clone();
                }
                // Its base deleted (a base per card: the active project no longer says it).
                let closing = base.as_deref().is_some_and(|id| qnc_ingest_base::closing(&at, id));
                if base.is_some() && (now != base || closing) {
                    let _ = send.send(Progress { left: true, ..Progress::default() });
                    return;
                }
                let read = progress(&at).unwrap_or_else(|error| {
                    // A base that cannot be read is said once in the log, not hidden as nothing done.
                    if error != said {
                        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-card-ingest progress not read: {error}"));
                        said = error;
                    }
                    Progress::default()
                });
                if send.send(read).is_err() {
                    return;
                }
                std::thread::sleep(READ);
            }
        });
        Ok(Self {
            root: root.to_path_buf(),
            application: IngestApplication::with_store_root(root)?,
            attached: qnc_attached_cards::AttachedCards::start(root),
            auto: qnc_card_autoselect::AutoSelect::new(Vec::new()),
            cameras: Vec::new(),
            cards: Vec::new(),
            linked: Default::default(),
            selected: false,
            making: false,
            progress: Progress::default(),
            read,
            looked: false,
            folder: None,
            preset: None,
            cards_too: true,
            card_cameras: Default::default(),
            only_card: None,
        })
    }

    /// One step: the cards attached, Ingest polled, the next action sent.
    pub fn poll(&mut self) -> Status {
        // A folder picked by the user: the cards attached are not this window's.
        let polled = self.attached.poll();
        // The camera of every card attached, for the browser's list of disks (also for a folder alone).
        if let Some(Ok(cards)) = &polled {
            self.card_cameras = cards.iter().filter_map(|card| Some((card.serial.to_ascii_lowercase(), card.camera.clone()?))).collect();
        }
        let cards = if self.cards_too { polled } else { None };
        // Only this window's card, when it has one.
        let only = self.only_card.clone();
        let cards = cards.map(|cards| cards.map(|cards| cards.into_iter().filter(|card| only.as_ref().is_none_or(|serial| card.serial.eq_ignore_ascii_case(serial))).collect::<Vec<_>>()));
        if let Some(Ok(cards)) = cards {
            self.looked = true;
            // A card an earlier base already has is not imported again: the base Ingest works in
            // links to that base (user 2026-11-13); only a card with clips that base has not is
            // Selected, and its clips that base has are left out (`qnc_ingest_base::not_taken`).
            for card in &cards {
                if let Some(elsewhere) = card.elsewhere.as_ref().filter(|_| !self.linked.contains(&card.serial)) {
                    let (root, serial, elsewhere) = (self.root.clone(), card.serial.clone(), elsewhere.clone());
                    std::thread::spawn(move || {
                        let line = match qnc_ingest_base::link_active_to(&root, &elsewhere.base_id) {
                            Ok(()) => format!("ai-card-ingest card {serial} known base={} had={} new={}: linked", elsewhere.base_label, elsewhere.had, elsewhere.new),
                            Err(error) => format!("ai-card-ingest card {serial} known base={}: not linked: {error}", elsewhere.base_label),
                        };
                        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
                    });
                    self.linked.insert(card.serial.clone());
                }
            }
            self.linked.retain(|serial| cards.iter().any(|card| &card.serial == serial));
            self.auto.want(&cards.iter().filter(|card| card.elsewhere.as_ref().is_none_or(|elsewhere| elsewhere.new > 0)).map(|card| card.serial.clone()).collect::<Vec<_>>());
            self.cards = cards
                .iter()
                .map(|card| CardLine { camera: card.camera.clone().unwrap_or_default(), known_in: card.elsewhere.as_ref().map(|elsewhere| elsewhere.base_label.clone()), new: card.elsewhere.as_ref().map_or(0, |elsewhere| elsewhere.new) })
                .collect();
            self.cameras = cards.into_iter().filter_map(|card| card.camera).collect();
        }
        while let Ok(progress) = self.read.try_recv() {
            self.progress = progress;
        }
        self.application.poll(None);
        if self.folder.is_some() {
            self.folder_step();
        }
        // While the user has the browser (a folder picked or Selected), no card is gone to.
        let browser_taken = matches!(self.folder, Some(FolderPick::Choosing { listed: true } | FolderPick::Taking(_) | FolderPick::Selecting));
        let next = if browser_taken { None } else { self.auto.next(self.application.view()) };
        if let Some(intent) = next {
            self.making = false;
            self.selected = true;
            self.application.dispatch(intent);
        }
        let view = self.application.view();
        // Every card Selected: all its clips go to the import (link by the project's settings).
        if self.selected && self.looked && !self.auto.busy() && !view.command_busy && !view.work_settings_loading {
            self.selected = false;
            self.application.dispatch(IngestIntent::empty(action_ids::INGEST_SELECT_ALL));
            // A base made to take only the new clips of a card another base has: what that base
            // has is not imported again (user 2026-11-12: "glupo je ponovo uvoziti ono što je uvezeno").
            match qnc_ingest_base::not_taken(&self.root) {
                Ok(not_taken) => {
                    let ticked: Vec<String> = self.application.view().clips.iter().filter(|clip| clip.selected && not_taken.contains(&clip.clip_id)).map(|clip| clip.clip_id.clone()).collect();
                    for clip_id in ticked {
                        self.application.dispatch(IngestIntent::new(action_ids::INGEST_CLIP_TOGGLE, qnc_ingest_application::IngestPayload::ClipId(clip_id)));
                    }
                }
                Err(error) => qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-card-ingest not taken not read: {error}")),
            }
            self.application.import_into_base();
            self.making = true;
        }
        // The import handed over: Ingest would go on to the next application; here nothing does.
        let _ = self.application.take_navigation_request();
        // Nothing is done before the cards were looked at once and every one was Selected.
        let selecting = !self.looked || self.auto.busy() || self.application.view().command_busy || self.selected || matches!(self.folder, Some(FolderPick::Taking(_) | FolderPick::Selecting));
        let phase = match () {
            _ if self.choosing() => Phase::Choosing,
            _ if selecting => Phase::Selecting,
            _ if self.progress.complete() => Phase::Done,
            _ if self.making || self.progress.clips > 0 => Phase::Making,
            _ => Phase::Waiting,
        };
        Status { phase, progress: self.progress, cameras: self.cameras.clone(), cards: self.cards.clone(), message: self.application.view().message.clone() }
    }
}

