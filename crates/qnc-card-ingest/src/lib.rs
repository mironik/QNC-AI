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

/// How often what is done is read from the base.
const READ: Duration = Duration::from_secs(1);

/// Where the ingest is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting for a card (or for the base to load).
    Waiting,
    /// A card is Selected: its clips are read into the base.
    Selecting,
    /// The worker copies the posters and makes the filmstrips and rough waves.
    Making,
    Done,
}

/// How many clips of the base have each thing made (user 2026-11-12: "progres bar po funkciji").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub clips: usize,
    pub posters: usize,
    pub filmstrips: usize,
    /// Rough or full.
    pub waves: usize,
}

impl Progress {
    /// Every clip has its poster, its filmstrip and a wave.
    pub fn complete(&self) -> bool {
        self.clips > 0 && self.posters >= self.clips && self.filmstrips >= self.clips && self.waves >= self.clips
    }
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub phase: Phase,
    pub progress: Progress,
    /// The cameras of the cards attached.
    pub cameras: Vec<String>,
    /// What Ingest said last (a failure among it).
    pub message: String,
}

pub struct CardIngest {
    application: IngestApplication,
    attached: qnc_attached_cards::AttachedCards,
    auto: qnc_card_autoselect::AutoSelect,
    cameras: Vec<String>,
    /// A Select ran since the clips were imported last.
    selected: bool,
    /// The import was handed to the worker.
    making: bool,
    progress: Progress,
    read: Receiver<Progress>,
    /// The attached cards were looked at once.
    looked: bool,
}

impl CardIngest {
    pub fn start(root: &Path) -> Result<Self, String> {
        let (send, read) = channel();
        let at = root.to_path_buf();
        std::thread::spawn(move || loop {
            if send.send(progress(&at).unwrap_or_default()).is_err() {
                return;
            }
            std::thread::sleep(READ);
        });
        Ok(Self {
            application: IngestApplication::with_store_root(root)?,
            attached: qnc_attached_cards::AttachedCards::start(root),
            auto: qnc_card_autoselect::AutoSelect::new(Vec::new()),
            cameras: Vec::new(),
            selected: false,
            making: false,
            progress: Progress::default(),
            read,
            looked: false,
        })
    }

    /// One step: the cards attached, Ingest polled, the next action sent.
    pub fn poll(&mut self) -> Status {
        if let Some(Ok(cards)) = self.attached.poll() {
            self.looked = true;
            self.auto.want(&cards.iter().map(|card| card.serial.clone()).collect::<Vec<_>>());
            self.cameras = cards.into_iter().filter_map(|card| card.camera).collect();
        }
        while let Ok(progress) = self.read.try_recv() {
            self.progress = progress;
        }
        self.application.poll(None);
        if let Some(intent) = self.auto.next(self.application.view()) {
            self.making = false;
            self.selected = true;
            self.application.dispatch(intent);
        }
        let view = self.application.view();
        // Every card Selected: all its clips go to the import (link by the project's settings).
        if self.selected && self.looked && !self.auto.busy() && !view.command_busy && !view.work_settings_loading {
            self.selected = false;
            self.application.dispatch(IngestIntent::empty(action_ids::INGEST_SELECT_ALL));
            self.application.import_into_base();
            self.making = true;
        }
        // The import handed over: Ingest would go on to the next application; here nothing does.
        let _ = self.application.take_navigation_request();
        // Nothing is done before the cards were looked at once and every one was Selected.
        let selecting = !self.looked || self.auto.busy() || self.application.view().command_busy || self.selected;
        let phase = match () {
            _ if selecting => Phase::Selecting,
            _ if self.progress.complete() => Phase::Done,
            _ if self.making || self.progress.clips > 0 => Phase::Making,
            _ => Phase::Waiting,
        };
        Status { phase, progress: self.progress, cameras: self.cameras.clone(), message: self.application.view().message.clone() }
    }
}

/// What is done in the active base of `root`, read from its database.
fn progress(root: &PathBuf) -> Result<Progress, String> {
    let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|error| error.to_string())?;
    let snapshot = active.read().map_err(|error| error.to_string())?;
    let content = qnc_content_read::ContentReader::for_project(active.settings_reader(), &snapshot.settings)?;
    let posters = content.clip_posters()?;
    let mut artifacts = qnc_artifact_db::ArtifactReader::open(&qnc_db_broker::ProjectDbTarget::for_project(active.settings_reader(), &snapshot.settings)?)?;
    let mut progress = Progress { clips: posters.len(), posters: posters.iter().filter(|(_, poster)| poster.is_some()).count(), ..Default::default() };
    for (clip, _) in &posters {
        if artifacts.read_filmstrip(clip)?.is_some_and(|record| record.status == "ready") {
            progress.filmstrips += 1;
        }
        if artifacts.read_wave(clip)?.is_some() {
            progress.waves += 1;
        }
    }
    Ok(progress)
}
