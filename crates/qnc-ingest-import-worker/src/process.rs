//! The project settings executed as a background process.
//!
//! Uvezi writes the selection to the database and puts the selected clips in the import
//! queue; then it makes sure `qnc-ingest-worker`, a separate application, is running.
//! The worker has no rules of its own. It reads the settings of the active project once,
//! takes the queued clips and does what the settings say for each (link: nothing is
//! copied; proxy or original: a copy, and the poster with it), and ends. Everything the
//! form and the worker need to know about each other lives in the project database:
//! the worker lease (only one runs), whether a player works (the copy waits) and what
//! the worker did last. The form never waits for the worker.

use crate::{run_next, ConfigMediaOpener, ImportQueue, TransportQueue};
use qnc_content_store::{ImportedCopy, StoredClip};
use qnc_media_metadata::MediaRepresentation;
use qnc_active_project_read::ActiveProjectReader;
use qnc_playback_activity::{Lease, Writer, WORKER, WORKER_FRESH_SECONDS, WORKER_RESULT};
use qnc_content_store::ContentTarget;
use qnc_ingest_work_plan::IngestWorkPlan;
use qnc_work_settings::SettingsReader;
use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

/// The name of the executable that runs beside the application.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSummary {
    pub imported: usize,
    pub failed: usize,
}

/// What one project needs to import.
struct Project {
    dir: PathBuf,
    queue: TransportQueue,
    opener: ConfigMediaOpener,
}

impl Project {
    fn open(
        root: &Path,
        reader: &SettingsReader,
        plan: &IngestWorkPlan,
        target: &ContentTarget,
        lease: Option<qnc_content_store::WriteLease>,
    ) -> Result<Self, String> {
        let dir = reader
            .local_workspace_dir(&plan.settings)
            .map_err(|e| e.to_string())?
            .ok_or(qnc_ui_text::text("import.import_needs_local_access_project_folder"))?;
        // Link needs no source; a copy from a source not attached fails per clip. A card
        // attached after the start is found when its clip comes (the sources read again).
        Ok(Self {
            dir,
            queue: TransportQueue::start_leased(target.clone(), lease)?,
            opener: ConfigMediaOpener::of_root(root, qnc_playback_activity::playback_pause(target.clone())),
        })
    }
}

/// The import of one Uvezi: the settings of the active project are read once, then the
/// queued clips are taken one by one until the queue is empty. Nothing happens when
/// another worker already holds the lease of this project.
pub fn run_service(root: &Path) -> Result<ImportSummary, String> {
    let active_project = ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
    let snapshot = active_project.read().map_err(|e| e.to_string())?;
    let reader = active_project.settings_reader().clone();
    let target = ContentTarget::for_project(&reader, &snapshot.settings)?;
    let Some(lease) = Lease::claim(target, WORKER, WORKER_FRESH_SECONDS)? else {
        return Ok(ImportSummary {
            imported: 0,
            failed: 0,
        });
    };
    run_import(root, &ImportRun { project_id: &snapshot.settings.project_id, lease: &lease, cancel: lease.lost() })
}

/// The run an import belongs to (external audit 2026-10-17): the project its lease was
/// taken for, the lease, and the cancel of the whole run.
pub struct ImportRun<'a> {
    pub project_id: &'a str,
    pub lease: &'a Lease,
    pub cancel: &'a AtomicBool,
}

impl ImportRun<'_> {
    /// The run may still work and publish: not cancelled and the lease still its own.
    fn allowed(&self) -> bool {
        !self.cancel.load(std::sync::atomic::Ordering::Relaxed) && self.lease.confirm()
    }
}

/// The queue of a run: it takes no clip and publishes no result once the run may no
/// longer (cancelled, or its lease is another's).
struct RunQueue<'q, 'r> {
    queue: &'q mut dyn ImportQueue,
    run: &'q ImportRun<'r>,
}

impl ImportQueue for RunQueue<'_, '_> {
    fn claim_next(&mut self) -> Result<Option<StoredClip>, String> {
        if !self.run.allowed() {
            return Ok(None);
        }
        self.queue.claim_next()
    }

    fn heartbeat(&mut self, clip_id: &str) -> Result<(), String> {
        self.queue.heartbeat(clip_id)
    }

    fn finish_import(
        &mut self,
        clip_id: String,
        media_uri: Option<String>,
        thumbnail_uri: Option<String>,
        copy_of: Option<ImportedCopy>,
        optimized: Option<MediaRepresentation>,
        error: Option<String>,
    ) -> Result<(), String> {
        if !self.run.allowed() {
            return Err(qnc_ui_text::text("import.lease_lost").into());
        }
        self.queue.finish_import(clip_id, media_uri, thumbnail_uri, copy_of, optimized, error)
    }
}

/// The import of one Uvezi under the lease the caller holds (the background application
/// also builds the artifacts under it), for the project the lease was taken for only.
pub fn run_import(root: &Path, run: &ImportRun<'_>) -> Result<ImportSummary, String> {
    let active_project = ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
    let snapshot = active_project.read().map_err(|e| e.to_string())?;
    if snapshot.settings.project_id != run.project_id {
        return Ok(ImportSummary { imported: 0, failed: 0 }); // another project is active now
    }
    let reader = active_project.settings_reader().clone();
    let plan = IngestWorkPlan::from_settings(snapshot.settings)?;
    let target = ContentTarget::for_project(&reader, &plan.settings)?;
    let mut summary = ImportSummary {
        imported: 0,
        failed: 0,
    };
    let mut project = Project::open(root, &reader, &plan, &target, Some(run.lease.write_lease()))?;
    // The first clip tells the pace of its card (user 2026-11-12: "može li u više niti"): its
    // medium is tested once and that many clips are copied side by side, each worker with its own
    // writer of the queue. Linked clips copy nothing, so one is enough for them.
    let first = RunQueue { queue: &mut project.queue, run }.claim_next()?;
    let workers = first.as_ref().map_or(1, |clip| copy_workers(&project.opener, clip, &plan, run.cancel));
    let counts = std::sync::Mutex::new((0_usize, 0_usize));
    let count = |outcome: &crate::Outcome| {
        let mut counts = counts.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if outcome.result.is_ok() { counts.0 += 1 } else { counts.1 += 1 }
    };
    let drain = |project: &mut Project, first: Option<StoredClip>| -> Result<(), String> {
        let mut queue = RunQueue { queue: &mut project.queue, run };
        if let Some(clip) = first {
            count(&crate::run::import_one(&mut queue, clip, &plan, &project.dir, &project.opener, run.cancel)?);
        }
        while let Some(outcome) = run_next(&mut queue, &plan, &project.dir, &project.opener, run.cancel)? {
            count(&outcome);
        }
        Ok(())
    };
    let outcome = std::thread::scope(|scope| {
        let others: Vec<_> = (1..workers)
            .map(|_| scope.spawn(|| Project::open(root, &reader, &plan, &target, Some(run.lease.write_lease())).and_then(|mut project| drain(&mut project, None))))
            .collect();
        let mine = drain(&mut project, first);
        others.into_iter().map(|other| other.join().unwrap_or_else(|_| Err(qnc_ui_text::text("import.import_interrupted").into()))).fold(mine, Result::and)
    });
    (summary.imported, summary.failed) = counts.into_inner().unwrap_or_else(|poisoned| poisoned.into_inner());
    let result = match &outcome {
        Ok(()) => qnc_ui_text::fill("import.result_done", &[("imported", &summary.imported.to_string()), ("failed", &summary.failed.to_string())]),
        Err(error) => qnc_ui_text::fill("import.result_error", &[("error", error)]),
    };
    if run.lease.confirm() {
        if let Ok(mut writer) = Writer::start(target) {
            let _ = writer.set(WORKER_RESULT, &result);
        }
    }
    outcome.map(|()| summary)
}

/// How many clips are made at once. A copy goes one clip at a time: the card is read in order at
/// its own speed, several copies only share it (measured 2026-11-12: one 40 MB/s, two 39.4 MB/s in
/// all) and the first clip, which the records, filmstrips, waves and AI wait for, is done later; the
/// random-read test of the card picked 8 once. An optimized copy (an encode) takes as many as the
/// test of the card (`qnc-read-pace`) reads fastest with, never more than the threads of this
/// computer; one when its medium cannot be tested (another computer's file, a small file).
fn copy_workers(opener: &ConfigMediaOpener, clip: &StoredClip, plan: &IngestWorkPlan, cancel: &AtomicBool) -> usize {
    if !matches!(crate::action_for(clip, plan), Ok(crate::Action::Optimize { .. })) {
        return 1;
    }
    let Some(file) = crate::MediaOpener::local_path(opener, &clip.clip.snapshot.binding.original_uri) else {
        return 1;
    };
    let most = std::thread::available_parallelism().map_or(1, usize::from);
    let readers = match qnc_read_pace::measure(&file, most, cancel) {
        Ok(Some(measured)) => measured.readers,
        _ => 1,
    };
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("import copy workers={readers} tested={}", file.display()));
    readers
}

/// Makes sure the worker runs (the wake block's; no copy of it here).
pub use qnc_worker_wake::{wake as launch_worker, WORKER_EXECUTABLE};

