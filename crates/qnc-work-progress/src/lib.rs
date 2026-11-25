//! What the background work of a project has done, function by function (user 2026-11-12:
//! "progres bar po funkciji"), and the X that stops a function the user does not need. Read from
//! the project's database (and the computer's AI stores) off the caller's thread; it draws nothing:
//! the wizard's import window and AI Media Assist's info box show it (`qnc-work-progress-view`,
//! user 2026-11-16: "treba izgled kao wizard, da korisnik može isključiti neke korake").

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// How often a [`Follower`] reads what is done.
const READ: Duration = Duration::from_secs(1);

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

    /// QNC can be started: every clip imported and its first channel heard (transcribed, stopped,
    /// or the transcript off); what is left (filmstrips, waves, AI) goes on in the background
    /// (user 2026-11-13: "čim je završen Prijepis učini dostupnim dugme Pokreni QNC").
    pub fn ready_for_qnc(&self) -> bool {
        let heard = match self.transcripts {
            Some(done) => done >= self.clips || self.is_stopped(qnc_stopped_work::TRANSCRIPTS),
            None => true,
        };
        self.clips > 0 && self.imported >= self.clips && heard
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
    stop_all(root, vec![function]);
}

/// Nothing stopped in the active base of `root` any more (a kept base goes on).
pub fn resume_all(root: &Path) -> Result<(), String> {
    let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|error| error.to_string())?;
    let snapshot = active.read().map_err(|error| error.to_string())?;
    qnc_stopped_work::resume(qnc_content_store::ContentTarget::for_project(active.settings_reader(), &snapshot.settings)?)
}

/// Stops every function of `functions` in the active base of `root`, one after the other on a
/// thread of its own (user 2026-11-16: Cancel of the wizard stops all that still runs).
pub fn stop_all(root: &Path, functions: Vec<&'static str>) {
    let root = root.to_path_buf();
    std::thread::spawn(move || {
        let target = (|| {
            let active = qnc_active_project_read::ActiveProjectReader::from_root(&root).map_err(|error| error.to_string())?;
            let snapshot = active.read().map_err(|error| error.to_string())?;
            qnc_content_store::ContentTarget::for_project(active.settings_reader(), &snapshot.settings)
        })();
        for function in functions {
            let result = match &target {
                Ok(target) => qnc_stopped_work::stop(target.clone(), function),
                Err(error) => Err(error.clone()),
            };
            let line = match result {
                Ok(()) => format!("ai-card-ingest stopped {function}"),
                Err(error) => format!("ai-card-ingest {function} not stopped: {error}"),
            };
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
        }
    });
}

/// What is done in the active base of `root`, read from its database.
pub fn progress(root: &PathBuf) -> Result<Progress, String> {
    let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|error| error.to_string())?;
    let not_taken = qnc_ingest_base::not_taken(root)?;
    progress_in(root, &active, &not_taken)
}

/// What is done in the base `id` of `root`, read from its database: for the tray, which asks once
/// for every base being ingested whether QNC is started (user 2026-11-14). Read only.
pub fn progress_of(root: &Path, id: &str) -> Result<Progress, String> {
    let settings = qnc_work_settings::SettingsReader::from_root(root).map_err(|error| error.to_string())?.with_project(id);
    let active = qnc_active_project_read::ActiveProjectReader::from_settings_reader(settings);
    let not_taken = qnc_ingest_base::not_taken_of(root, id)?;
    progress_in(&root.to_path_buf(), &active, &not_taken)
}

fn progress_in(root: &PathBuf, active: &qnc_active_project_read::ActiveProjectReader, not_taken: &std::collections::BTreeSet<String>) -> Result<Progress, String> {
    let snapshot = active.read().map_err(|error| error.to_string())?;
    let content = qnc_content_read::ContentReader::for_project(active.settings_reader(), &snapshot.settings)?;
    let settings = &snapshot.settings;
    // A base that takes only the new clips counts only them (the rest stays in the catalog, not imported).
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
    // Only the ids of what is made: reading every record parsed every wave's peaks (the worker's
    // conductor counts every 2 s).
    let (filmstrips, waves) = artifacts.read_made()?;
    let (filmstrips, waves): (std::collections::BTreeSet<String>, std::collections::BTreeSet<String>) = (filmstrips.into_iter().collect(), waves.into_iter().collect());
    for (clip, _) in &posters {
        if let Some(done) = progress.filmstrips.as_mut() {
            *done += usize::from(filmstrips.contains(clip));
        }
        if let Some(done) = progress.waves.as_mut() {
            *done += usize::from(waves.contains(clip));
        }
    }
    // The first channel heard, and which clips it shows to be statements: the AI shots leave
    // those alone (user 2026-11-12), so they count as done on the AI bars.
    let mut statements = std::collections::BTreeSet::new();
    if feature("transcription") {
        let frames: std::collections::HashMap<String, u64> = content.summaries()?.into_iter().map(|clip| (clip.clip_id, clip.duration_frames)).collect();
        // A clip is heard when its transcript is in the base, or its transcription failed. Not by
        // the queue alone: AI Media Assist takes the done rows out of it (live 2026-11-15: the bar
        // went back to 0 once it was opened).
        let mut heard: std::collections::BTreeSet<String> = qnc_transcript_db::read_transcribed(&target)?.into_iter().collect();
        let queue = qnc_transcript_db::read_queue(&target)?;
        for entry in &queue {
            if matches!(entry.state, qnc_transcript_db::QueueState::Done | qnc_transcript_db::QueueState::Failed) {
                heard.insert(entry.clip_id.clone());
            }
        }
        // A long clip is heard in pieces: it is heard once none of them waits or runs. A fine pass
        // of a clip already heard (the Croatian model after Parakeet) does not take it back
        // (live 2026-11-19: the bar went 106, 105, 106 as AI Media Assist refined clip by clip).
        for entry in &queue {
            if entry.quality == qnc_transcript_db::QueueQuality::Fast && matches!(entry.state, qnc_transcript_db::QueueState::Waiting | qnc_transcript_db::QueueState::Running) {
                heard.remove(&entry.clip_id);
            }
        }
        let clips: std::collections::BTreeSet<&String> = posters.iter().map(|(clip, _)| clip).collect();
        heard.retain(|clip| clips.contains(clip));
        let heard_ids: Vec<String> = heard.iter().cloned().collect();
        for (clip, transcripts) in qnc_transcript_db::read_clips(&target, &heard_ids)? {
            let to = frames.get(&clip).copied().unwrap_or(0);
            if qnc_transcript_db::spoken_share(&transcripts, 0, to).is_some_and(|share| share >= qnc_transcript_db::SPOKEN) {
                statements.insert(clip.clone());
            }
        }
        progress.transcripts = Some(heard.len());
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
    // No captioner on this computer (user 2026-11-25: Florence-2 is of no use): no captions counted.
    if let Some(captioner) = feature("shot_caption").then(qnc_ai_catalog::installed_shot_captioner).and_then(Result::ok) {
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

    #[test]
    fn qnc_can_start_once_every_clip_is_imported_and_heard_the_rest_goes_on() {
        let mut progress = Progress { clips: 4, imported: 4, posters: 4, filmstrips: Some(1), waves: Some(0), transcripts: Some(3), shots: Some(0), ..Default::default() };
        assert!(!progress.ready_for_qnc(), "a clip is not heard yet");
        progress.transcripts = Some(4);
        assert!(progress.ready_for_qnc() && !progress.complete(), "heard: QNC can start, the rest goes on");
        progress.imported = 3;
        assert!(!progress.ready_for_qnc(), "a clip is not imported yet");
        progress = Progress { clips: 2, imported: 2, transcripts: None, ..Default::default() };
        assert!(progress.ready_for_qnc(), "the transcript off: imported is enough");
    }
}

/// What is done in the active project of `root`, read once a second on a thread of its own while
/// the follower lives.
pub struct Follower {
    read: Receiver<Progress>,
    last: Option<Progress>,
}

impl Follower {
    pub fn start(root: &Path) -> Self {
        let (send, read) = channel();
        let root = root.to_path_buf();
        let spawned = std::thread::Builder::new().name("qnc-work-progress".into()).spawn(move || {
            let mut said = String::new();
            loop {
                let read = progress(&root).unwrap_or_else(|error| {
                    if error != said {
                        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("work progress not read: {error}"));
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
        if spawned.is_err() {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, "work progress: no thread");
        }
        Self { read, last: None }
    }

    /// What was read last; true when it changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(read) = self.read.try_recv() {
            changed |= self.last.as_ref() != Some(&read);
            self.last = Some(read);
        }
        changed
    }

    pub fn last(&self) -> Option<&Progress> {
        self.last.as_ref()
    }
}
