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

/// How many clips of the base have each thing made (user 2026-11-12: "progres bar po funkciji";
/// "ako je u postavkama uključena i analize, i one se rade.. sve se treba prikazivati u infu").
/// A function the project's settings leave off is `None` and has no bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Progress {
    pub clips: usize,
    /// Imported by the project's settings (copied into the project, or linked).
    pub imported: usize,
    /// Whose import failed (a card taken out): they go again when the card is back.
    pub failed: usize,
    pub posters: usize,
    pub filmstrips: Option<usize>,
    /// Rough or full.
    pub waves: Option<usize>,
    /// AI: the shots of the clip analysed.
    pub shots: Option<usize>,
    /// AI: what the shots show.
    pub content: Option<usize>,
    /// AI: a sentence about each shot.
    pub captions: Option<usize>,
    /// The functions the user stopped (the X beside a bar), one bit per `qnc_stopped_work::ALL`.
    pub stopped: u8,
}

/// One bar of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bar {
    /// The code of its text.
    pub text: &'static str,
    /// The function (`qnc_stopped_work`), what its X stops.
    pub function: &'static str,
    pub done: usize,
    pub stopped: bool,
}

impl Progress {
    /// Every clip imported and every function the settings have on made for it; a function the
    /// user stopped is not waited for.
    pub fn complete(&self) -> bool {
        self.clips > 0 && self.bars().iter().all(|bar| bar.stopped || bar.done >= self.clips)
    }

    /// Whether the user stopped `function`.
    pub fn is_stopped(&self, function: &str) -> bool {
        qnc_stopped_work::ALL.iter().position(|code| *code == function).is_some_and(|bit| self.stopped & (1 << bit) != 0)
    }

    /// One bar per function the project's settings have on.
    pub fn bars(&self) -> Vec<Bar> {
        use qnc_stopped_work::{CAPTIONS, CONTENT, FILMSTRIPS, IMPORT, POSTERS, SHOTS, WAVES};
        [
            ("card_ingest.imported", IMPORT, Some(self.imported)),
            ("card_ingest.posters", POSTERS, Some(self.posters)),
            ("card_ingest.filmstrips", FILMSTRIPS, self.filmstrips),
            ("card_ingest.waves", WAVES, self.waves),
            ("card_ingest.shots", SHOTS, self.shots),
            ("card_ingest.content", CONTENT, self.content),
            ("card_ingest.captions", CAPTIONS, self.captions),
        ]
        .into_iter()
        .filter_map(|(text, function, done)| Some(Bar { text, function, done: done?, stopped: self.is_stopped(function) }))
        .collect()
    }
}

/// Stops `function` in the active base of `root` (the X beside its bar), on a thread of its own;
/// the next reading of the progress shows it stopped.
pub fn stop(root: &Path, function: &'static str) {
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let result = (|| {
            let active = qnc_active_project_read::ActiveProjectReader::from_root(&root).map_err(|error| error.to_string())?;
            let snapshot = active.read().map_err(|error| error.to_string())?;
            let target = qnc_content_store::ContentTarget::for_project(active.settings_reader(), &snapshot.settings)?;
            qnc_stopped_work::stop(target, function)
        })();
        let line = match result {
            Ok(()) => format!("ai-card-ingest stopped {function}"),
            Err(error) => format!("ai-card-ingest {function} not stopped: {error}"),
        };
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
    });
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
        std::thread::spawn(move || {
            let mut said = String::new();
            loop {
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
    let settings = &snapshot.settings;
    let posters = content.clip_posters()?;
    let target = qnc_db_broker::ProjectDbTarget::for_project(active.settings_reader(), settings)?;
    let mut artifacts = qnc_artifact_db::ArtifactReader::open(&target)?;
    let content_target = qnc_content_store::ContentTarget::for_project(active.settings_reader(), settings)?;
    let stopped = qnc_stopped_work::read(&content_target);
    let stopped = qnc_stopped_work::ALL.iter().enumerate().filter(|(_, code)| stopped.contains(**code)).fold(0u8, |bits, (bit, _)| bits | 1 << bit);
    let on = |kind| settings.artifact_mode(kind).map(|mode| mode != qnc_work_settings::ArtifactMode::Off).unwrap_or(true);
    let feature = |code| settings.ai_feature(code).unwrap_or(false);
    let mut progress = Progress {
        clips: posters.len(),
        posters: posters.iter().filter(|(_, poster)| poster.is_some()).count(),
        filmstrips: on(qnc_work_settings::ArtifactKind::Filmstrip).then_some(0),
        waves: on(qnc_work_settings::ArtifactKind::Wave).then_some(0),
        stopped,
        ..Default::default()
    };
    for summary in content.summaries()? {
        match summary.import_status.as_str() {
            "imported" | "done" => progress.imported += 1,
            "failed" => progress.failed += 1,
            _ => {}
        }
    }
    for (clip, _) in &posters {
        if let Some(done) = progress.filmstrips.as_mut() {
            *done += usize::from(artifacts.read_filmstrip(clip)?.is_some_and(|record| record.status == "ready"));
        }
        if let Some(done) = progress.waves.as_mut() {
            *done += usize::from(artifacts.read_wave(clip)?.is_some());
        }
    }
    // The analyses the project's AI has on, counted per clip (user 2026-11-12).
    if feature("shot_analysis") {
        progress.shots = Some(qnc_shot_db::read_analysed(&target).map_err(|error| error.to_string())?.len());
    }
    if feature("shot_content") {
        let engine = qnc_ai_catalog::installed_shot_content()?;
        // Only the look at the shots counts: the quick look at the card's posters right after Select
        // is not the content of the clip (live 2026-11-12: 30/30 with 9 clips copied).
        let clips: std::collections::BTreeSet<_> = qnc_shot_content_db::read_all(&target, &engine.model_name()).map_err(|error| error.to_string())?.into_iter().filter(|content| content.parts.iter().any(|part| part.kind != "poster")).map(|content| content.clip_id).collect();
        progress.content = Some(clips.len());
    }
    if feature("shot_caption") {
        let captioner = qnc_ai_catalog::installed_shot_captioner()?;
        let keys = qnc_shot_captions::captioned(root, &captioner.model_name(), &snapshot.project_uri)?;
        let clips: std::collections::BTreeSet<_> = keys.iter().filter_map(|key| key.split('|').nth(1)).collect();
        progress.captions = Some(clips.len());
    }
    Ok(progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_function_the_user_stopped_is_not_waited_for() {
        let mut progress = Progress { clips: 4, imported: 4, posters: 4, filmstrips: Some(1), waves: Some(4), ..Default::default() };
        assert!(!progress.complete());
        let bit = qnc_stopped_work::ALL.iter().position(|code| *code == qnc_stopped_work::FILMSTRIPS).unwrap();
        progress.stopped = 1 << bit;
        assert!(progress.complete());
        let bar = progress.bars().into_iter().find(|bar| bar.function == qnc_stopped_work::FILMSTRIPS).unwrap();
        assert!(bar.stopped && bar.done == 1);
        assert!(!progress.is_stopped(qnc_stopped_work::WAVES));
    }
}
