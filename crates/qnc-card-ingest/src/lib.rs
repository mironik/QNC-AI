//! The background ingest of the cards (user 2026-11-12: "globalna baza kao ingest na velikim
//! ingest sustavima: korisnik ubaci karticu, tray provjeri poznaje li karticu i odmah radi Select
//! All i ingestira materijal po osnovnim postavkama (link media); ingest proces koji pokreće tray
//! je kopiranje postera, kreiranje filmstripa i grube wave"). It drives the Ingest application
//! without a form, through Ingest's own actions, as the user would: every attached card of a known
//! camera is Selected into the global Ingest base, all its clips are selected and imported by the
//! project's settings (link: nothing is copied); the background worker copies the posters and
//! makes the filmstrips and rough waves. It is done when nothing is left to Select and the worker
//! has stopped. It draws nothing: a window shows `status()`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qnc_ingest_application::{action_ids, IngestApplication, IngestIntent};

/// How long the worker must be gone before the ingest counts as done.
const QUIET: Duration = Duration::from_secs(4);

/// Where the ingest is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for a card (or for the base to load).
    Waiting,
    /// A card is Selected: its clips are read into the base.
    Selecting,
    /// The worker copies the posters and makes the filmstrips and waves.
    Making,
    Done,
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    /// The clips of the base now.
    pub clips: usize,
    /// The cameras of the cards attached.
    pub cameras: Vec<String>,
    /// What Ingest said last (a failure among it).
    pub message: String,
}

pub struct CardIngest {
    root: PathBuf,
    application: IngestApplication,
    attached: qnc_attached_cards::AttachedCards,
    auto: qnc_card_autoselect::AutoSelect,
    cameras: Vec<String>,
    /// A Select ran since the clips were imported last.
    selected: bool,
    /// The import was handed to the worker; since when the worker was last seen gone.
    making: Option<Option<Instant>>,
    seen_worker: Option<Instant>,
}

impl CardIngest {
    pub fn start(root: &Path) -> Result<Self, String> {
        Ok(Self {
            root: root.to_path_buf(),
            application: IngestApplication::with_store_root(root)?,
            attached: qnc_attached_cards::AttachedCards::start(root),
            auto: qnc_card_autoselect::AutoSelect::new(Vec::new()),
            cameras: Vec::new(),
            selected: false,
            making: None,
            seen_worker: None,
        })
    }

    /// One step: the cards attached, Ingest polled, the next action sent.
    pub fn poll(&mut self) -> Status {
        if let Some(Ok(cards)) = self.attached.poll() {
            self.auto.want(&cards.iter().map(|card| card.serial.clone()).collect::<Vec<_>>());
            self.cameras = cards.into_iter().filter_map(|card| card.camera).collect();
        }
        self.application.poll(None);
        if let Some(intent) = self.auto.next(self.application.view()) {
            self.making = None;
            self.selected = true;
            self.application.dispatch(intent);
        }
        let view = self.application.view();
        let selecting = self.auto.busy() || view.command_busy || view.work_settings_loading;
        // Every card Selected: all its clips go to the import (link by the project's settings).
        if self.selected && !selecting {
            self.selected = false;
            self.application.dispatch(IngestIntent::empty(action_ids::INGEST_SELECT_ALL));
            self.application.import_into_base();
            self.making = Some(None);
            self.seen_worker = Some(Instant::now());
        }
        // The import handed over: Ingest would go on to the next application; here nothing does.
        let _ = self.application.take_navigation_request();
        let phase = match self.making {
            _ if selecting => Phase::Selecting,
            Some(_) if self.worker_running() => {
                self.seen_worker = Some(Instant::now());
                Phase::Making
            }
            Some(_) if self.seen_worker.is_some_and(|at| at.elapsed() < QUIET) => Phase::Making,
            Some(_) => Phase::Done,
            None => Phase::Waiting,
        };
        let view = self.application.view();
        Status { phase, clips: view.clips.len(), cameras: self.cameras.clone(), message: view.message.clone() }
    }

    /// Whether the background worker of the base holds its lease now.
    fn worker_running(&self) -> bool {
        let Ok(active) = qnc_active_project_read::ActiveProjectReader::from_root(&self.root) else { return false };
        let Ok(snapshot) = active.read() else { return false };
        let Ok(target) = qnc_content_store::ContentTarget::for_project(active.settings_reader(), &snapshot.settings) else { return false };
        qnc_playback_activity::is_fresh(&target, qnc_playback_activity::WORKER, qnc_playback_activity::WORKER_FRESH_SECONDS)
    }
}
