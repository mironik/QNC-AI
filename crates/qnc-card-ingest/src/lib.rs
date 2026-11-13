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
    /// A folder is to be picked in Ingest's browser (a source that is no card).
    Choosing,
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
    /// The first channel heard (transcribed, or found to have no speech to hear).
    pub transcripts: Option<usize>,
    /// Clips whose first channel holds a statement: the AI shots leave them alone.
    pub statements: usize,
    /// The functions the user stopped (the X beside a bar), one bit per `qnc_stopped_work::ALL`.
    pub stopped: u8,
    /// The base is no longer the active project (closed to be deleted, another opened).
    pub left: bool,
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
        use qnc_stopped_work::{CAPTIONS, CONTENT, FILMSTRIPS, IMPORT, POSTERS, SHOTS, TRANSCRIPTS, WAVES};
        [
            ("card_ingest.imported", IMPORT, Some(self.imported)),
            ("card_ingest.posters", POSTERS, Some(self.posters)),
            ("card_ingest.filmstrips", FILMSTRIPS, self.filmstrips),
            ("card_ingest.waves", WAVES, self.waves),
            ("card_ingest.transcripts", TRANSCRIPTS, self.transcripts),
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
    /// The cards attached are ingested too (not when the window was started for a folder alone).
    cards_too: bool,
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
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-card-ingest folder {uri}"));
                self.folder = Some(FolderPick::Taking(uri.clone()));
            }
        }
        self.application.dispatch(intent);
    }

    /// The folder's Select, step by step: taken, running, ended (then the clips go to the import).
    fn folder_step(&mut self) {
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
                if base.is_some() && now != base {
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
            cards_too: true,
        })
    }

    /// One step: the cards attached, Ingest polled, the next action sent.
    pub fn poll(&mut self) -> Status {
        // A folder picked by the user: the cards attached are not this window's.
        let cards = if self.cards_too { self.attached.poll() } else { None };
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

/// What is done in the active base of `root`, read from its database.
fn progress(root: &PathBuf) -> Result<Progress, String> {
    let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|error| error.to_string())?;
    let snapshot = active.read().map_err(|error| error.to_string())?;
    let content = qnc_content_read::ContentReader::for_project(active.settings_reader(), &snapshot.settings)?;
    let settings = &snapshot.settings;
    // A base that takes only the new clips counts only them (the rest stays in the catalog, not imported).
    let not_taken = qnc_ingest_base::not_taken(root)?;
    let posters: Vec<_> = content.clip_posters()?.into_iter().filter(|(clip, _)| !not_taken.contains(clip)).collect();
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
    // The first channel heard, and which clips it shows to be statements: the AI shots leave
    // those alone (user 2026-11-12), so they count as done on the AI bars.
    let mut statements = std::collections::BTreeSet::new();
    if feature("transcription") {
        let frames: std::collections::HashMap<String, u64> = content.summaries()?.into_iter().map(|clip| (clip.clip_id, clip.duration_frames)).collect();
        let mut heard = 0;
        for entry in qnc_transcript_db::read_queue(&target)? {
            match entry.state {
                qnc_transcript_db::QueueState::Done => {
                    heard += 1;
                    let transcripts = qnc_transcript_db::read_clip(&target, &entry.clip_id)?;
                    let to = frames.get(&entry.clip_id).copied().unwrap_or(entry.to_frame);
                    if qnc_transcript_db::spoken_share(&transcripts, 0, to).is_some_and(|share| share >= qnc_transcript_db::SPOKEN) {
                        statements.insert(entry.clip_id);
                    }
                }
                qnc_transcript_db::QueueState::Failed => heard += 1,
                _ => {}
            }
        }
        progress.transcripts = Some(heard);
        progress.statements = statements.len();
    }
    // The analyses the project's AI has on, counted per clip (user 2026-11-12).
    if feature("shot_analysis") {
        let analysed: std::collections::BTreeSet<String> = qnc_shot_db::read_analysed(&target).map_err(|error| error.to_string())?.into_iter().collect();
        progress.shots = Some(analysed.union(&statements).count());
    }
    if feature("shot_content") {
        let engine = qnc_ai_catalog::installed_shot_content()?;
        // Only the look at the shots counts: the quick look at the card's posters right after Select
        // is not the content of the clip (live 2026-11-12: 30/30 with 9 clips copied).
        let clips: std::collections::BTreeSet<_> = qnc_shot_content_db::read_all(&target, &engine.model_name()).map_err(|error| error.to_string())?.into_iter().filter(|content| content.parts.iter().any(|part| part.kind != "poster")).map(|content| content.clip_id).collect();
        progress.content = Some(clips.union(&statements).count());
    }
    if feature("shot_caption") {
        let captioner = qnc_ai_catalog::installed_shot_captioner()?;
        let keys = qnc_shot_captions::captioned(root, &captioner.model_name(), &snapshot.project_uri)?;
        let clips: std::collections::BTreeSet<_> = keys.iter().filter_map(|key| key.split('|').nth(1)).collect();
        let statements: std::collections::BTreeSet<&str> = statements.iter().map(String::as_str).collect();
        progress.captions = Some(clips.union(&statements).count());
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
