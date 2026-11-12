//! Background application started by Select and by Uvezi. It reads the settings of the
//! active project once, takes the queued clips of the selection and does what the
//! settings say (link copies nothing, proxy or original copies), completes the card
//! records that playback still lacks something of (v5 media probe), builds the
//! artifacts, then ends. Its lease, the pause while a
//! player works and its result are kept in the project database, nowhere else.

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

fn main() -> ExitCode {
    qnc_dev_diagnostics::log_panics("qnc-ingest-worker");
    let Some(root) = root_argument() else {
        eprintln!("Upotreba: qnc-ingest-worker --root <QNC korijen>");
        return ExitCode::from(2);
    };
    match run_background(&root) {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("pozadinski proces nije uspio: {error}");
            ExitCode::from(1)
        }
    }
}

/// Runs while it holds the lease; then, with the lease released, looks at the project
/// database once more: an Uvezi or a Select whose start found the lease held (and ended at
/// once) is never lost, its clips are taken here (user 2026-10-18: a second card's records
/// were never completed). Records that wait again unchanged after a run (their completion
/// failed) do not start another one.
fn run_background(root: &Path) -> Result<(), String> {
    let mut last_records: Vec<String> = Vec::new();
    while run_once(root)? {
        if import_waiting(root)? {
            continue;
        }
        let records = records_waiting(root)?;
        if records.is_empty() || records == last_records {
            break;
        }
        last_records = records;
    }
    Ok(())
}

/// The clips of the active project whose record still waits for its completion.
fn records_waiting(root: &Path) -> Result<Vec<String>, String> {
    let active_project = qnc_active_project_read::ActiveProjectReader::from_root(root)
        .map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    qnc_content_read::ContentReader::for_project(active_project.settings_reader(), &snapshot.settings)?.records_waiting()
}

/// Whether clips wait in the import queue of the active project.
fn import_waiting(root: &Path) -> Result<bool, String> {
    let active_project = qnc_active_project_read::ActiveProjectReader::from_root(root)
        .map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    if import_stopped(&active_project, &snapshot.settings) {
        return Ok(false);
    }
    let reader = qnc_content_read::ContentReader::for_project(
        active_project.settings_reader(),
        &snapshot.settings,
    )?;
    Ok(reader
        .summaries()?
        .iter()
        .any(|clip| clip.import_status == "queued"))
}

/// Whether the import still has clips to copy (queued or being copied).
fn import_busy(root: &Path) -> Result<bool, String> {
    let active_project = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    if import_stopped(&active_project, &snapshot.settings) {
        return Ok(false);
    }
    let reader = qnc_content_read::ContentReader::for_project(active_project.settings_reader(), &snapshot.settings)?;
    Ok(reader.summaries()?.iter().any(|clip| matches!(clip.import_status.as_str(), "queued" | "processing")))
}

/// The import queue as the project database has it, looked at every second while the
/// other work of this run goes on (Uvezi may come at any time) and once more when it
/// has ended; an import runs only when clips wait, so its result stays the last real one.
fn import_while(
    root: &Path,
    running: &std::sync::atomic::AtomicBool,
    closed: &std::sync::atomic::AtomicBool,
    run: &qnc_ingest_import_worker::ImportRun<'_>,
) -> Result<(), String> {
    loop {
        if closed.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }
        let last = !running.load(std::sync::atomic::Ordering::Relaxed);
        if import_waiting(root)? {
            qnc_ingest_import_worker::run_import(root, run)?;
        }
        if last {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// One run under the lease; false when another worker holds it.
fn run_once(root: &Path) -> Result<bool, String> {
    let active_project = qnc_active_project_read::ActiveProjectReader::from_root(root)
        .map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    let started = snapshot.settings.project_id.clone();
    // The template copies the clips: only the copy reads the card, the rest works from the copies.
    let copying = snapshot.settings.ingest_copies();
    let reader = active_project.settings_reader().clone();
    let target = qnc_content_store::ContentTarget::for_project(&reader, &snapshot.settings)?;
    // One worker per project: the lease covers the import and the artifacts, so a
    // second start while the artifacts are still built finds it and ends at once.
    let player_works = qnc_playback_activity::playback_pause(target.clone());
    let stop_target = target.clone();
    let stops = Stops::new();
    stops.read(&stop_target);
    // Taken in one statement with its owner (external audit 2026-10-17): two starts never
    // both run; a run whose lease another took stops.
    let Some(lease) =
        qnc_playback_activity::Lease::claim(target, qnc_playback_activity::WORKER, qnc_playback_activity::WORKER_FRESH_SECONDS)?
    else {
        return Ok(false);
    };
    let running = std::sync::atomic::AtomicBool::new(true);
    // v5 media probe job: a card record gets what playback lacks, once, in the
    // background, so a preview of a clip not yet imported plays. Beside the artifacts
    // only the clip a preview wants; the others after them, several at once.
    let config = qnc_ingest_select::selection_config::SelectionConfig::load(root)
        .map_err(|error| error.to_string())?;
    let make_backend = |source_uri: &str, media: &[qnc_source_reader::SourceReference], copies: &[(String, PathBuf)]| {
        let card = config.sources.iter().find(|source| source.location.uri == source_uri);
        // Every medium probed has its copy in the project: the copy is read, never the card
        // (user 2026-11-12); the probe of this computer is the card's, or any local one.
        let copied: Option<Vec<(String, PathBuf)>> =
            media.iter().map(|reference| copies.iter().find(|(uri, _)| *uri == reference.uri()).cloned()).collect();
        if let Some(files) = copied.filter(|files| !files.is_empty()) {
            let local = card.into_iter().chain(&config.sources).find(|source| matches!(source.probe, qnc_ingest_select::selection_config::ProbeConfig::Local { .. }));
            return local
                .ok_or_else(|| qnc_ui_text::text("worker.source_not_connected").to_string())?
                .backend_with_files(&files)
                .map_err(|error| error.to_string());
        }
        card.ok_or_else(|| qnc_ui_text::text("worker.source_not_connected").to_string())?
            .backend(media)
            .map_err(|error| error.to_string())
    };
    let cancel = std::sync::atomic::AtomicBool::new(false);
    // The card sets the pace (user 2026-10-15: waves as soon as possible): first the
    // records (a wave needs its clip's final record), each clip's wave as soon as its record
    // is final, the filmstrip after them. The completion never gives way to the filmstrip.
    let completing = std::sync::atomic::AtomicBool::new(true);
    // The project database is the truth: once its active project is no longer the one
    // this run started for (closed, or another opened), the run stops and lets its
    // database go, so the closed project can be deleted.
    let closed = std::sync::atomic::AtomicBool::new(false);
    // The import belongs to this run: its project, its lease, its cancel.
    // Its cancel is the import's stop: the user's X, or the project closed.
    let run = qnc_ingest_import_worker::ImportRun { project_id: &started, lease: &lease, cancel: stops.flag(qnc_stopped_work::IMPORT) };
    let artifacts_done = std::sync::atomic::AtomicBool::new(false);
    // Kept across the phases of a run: the records a round left waiting, and whether the first
    // round (posters and the AI look at them) is done.
    let stuck_records = std::sync::Mutex::new(std::collections::BTreeSet::<String>::new());
    let first_round = std::sync::atomic::AtomicBool::new(true);
        // The records are completed for as long as this run works: a Select made while it
        // runs (or right after the worker was started for an opened project) adds clips
        // whose records are completed at once (user 2026-10-18: they waited until the
        // filmstrips were done, Play said NotFinal).
        let completion_work = || {
          // The records a round left waiting (its probe failed or was uncertain): only another
          // clip starts a new round (audit 2026-10-25: they started one every second).
          let mut stuck = stuck_records.lock().expect("stuck records");
          let mut first = first_round.swap(false, std::sync::atomic::Ordering::Relaxed);
          loop {
            let done = qnc_record_completion::complete_active_project(
                root,
                &make_backend,
                PROBE_WORKERS,
                &*player_works,
                &cancel,
                Some(lease.write_lease()),
            );
            let mut completed = first;
            match done {
                Ok(done) => {
                    completed |= !done.completed.is_empty();
                    for (clip, error) in done.failed {
                        eprintln!("zapis klipa {clip} nije dovrsen: {error}");
                        // Also where the logs are read (live 2026-10-22: an FX3A record failed unseen).
                        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("record-completion failed clip={clip} error={error}"));
                        stuck.insert(clip);
                    }
                }
                Err(error) => {
                    eprintln!("dovrsetak zapisa nije uspio: {error}");
                    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("record-completion failed error={error}"));
                }
            }
            if completed {
                trace("completion round completed records".into());
            }
            first = false;
            // Once more now the records are final: a clip whose record had no timing at
            // Select gets its poster from the start of the clip.
            if completed && !closed.load(std::sync::atomic::Ordering::Relaxed) {
                trace("completion: posters".into());
                make_posters(root, &config, &lease, &stops);
                // The AI looks at the posters right away (user 2026-11-10), before the waves,
                // the filmstrips and the import.
                // While the clips are copied the AI thread does it (live 2026-11-12: both looked
                // at the same 30 posters at once, 27 s of work twice).
                if !copying {
                    trace("completion: shots of the posters".into());
                    make_shots(root, &config, &lease, &stops, Look::Posters);
                    trace("completion: shots of the posters done".into());
                }
            }
            // While the clips are copied, every clip copied is worked on at once, from its copy
            // (user 2026-11-12: "nema čekanja dok se kartica ingestira"): the artifacts keep
            // taking the new clips and the records of the clips just copied are completed.
            completing.store(copying && import_busy(root).unwrap_or(false), std::sync::atomic::Ordering::Relaxed);
            // Until the artifacts are done, new records are looked for once a second.
            loop {
                let stop = closed.load(std::sync::atomic::Ordering::Relaxed)
                    || artifacts_done.load(std::sync::atomic::Ordering::Relaxed);
                if stop {
                    return;
                }
                std::thread::sleep(Duration::from_secs(1));
                let importing = copying && import_busy(root).unwrap_or(false);
                completing.store(importing, std::sync::atomic::Ordering::Relaxed);
                let waiting = records_waiting(root);
                if waiting.is_ok_and(|waiting| waiting.iter().any(|clip| !stuck.contains(clip))) {
                    completing.store(true, std::sync::atomic::Ordering::Relaxed);
                    break;
                }
            }
          }
        };
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                if !still_active(&active_project, &started) || !lease.held() {
                    closed.store(true, std::sync::atomic::Ordering::Relaxed);
                    cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                    stops.all();
                    return;
                }
                stops.read(&stop_target);
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        let import = scope.spawn(|| import_while(root, &running, &closed, &run));
        // The AI of each clip as soon as it is copied, from the copy, beside the copy and the
        // artifacts (user 2026-11-12: "ne rade AI, šta čekaju"); what is left after the import
        // is done once more at the end of the run.
        if copying {
            for look in [Look::Shots, Look::Captions] {
                let (running, closed, config, lease, stops) = (&running, &closed, &config, &lease, &stops);
                scope.spawn(move || {
                    while running.load(std::sync::atomic::Ordering::Relaxed) && !closed.load(std::sync::atomic::Ordering::Relaxed) {
                        make_shots(root, config, lease, stops, look);
                        for _ in 0..5 {
                            if !running.load(std::sync::atomic::Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_secs(1));
                        }
                    }
                });
            }
        }
        // The records first: every clip can play (and editing start) before the card is read
        // for waves and filmstrips (user 2026-10-18: speed for editing comes first).
        // When the clips are copied the artifacts start at once: they take each clip once it is
        // copied and its record final, from the copy.
        //
        // While the clips are copied the records and artifacts go round again for as long as
        // the import works (live 2026-11-12: the worker started at Select, before the import
        // was queued; its records and artifacts found nothing copied and ended, and the run then
        // only waited for the whole copy). The first seconds count as importing: the import is
        // queued right after Select.
        let grace = std::time::Instant::now() + Duration::from_secs(15);
        let mut artifacts = Ok(());
        loop {
            completing.store(true, std::sync::atomic::Ordering::Relaxed);
            artifacts_done.store(false, std::sync::atomic::Ordering::Relaxed);
            let completion = scope.spawn(&completion_work);
            while !copying && completing.load(std::sync::atomic::Ordering::Relaxed) && !closed.load(std::sync::atomic::Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(100));
            }
            artifacts = artifacts.and(run_artifacts(root, &lease, &closed, &completing, &stops));
            artifacts_done.store(true, std::sync::atomic::Ordering::Relaxed);
            let _ = completion.join();
            let importing = copying
                && !stops.is(qnc_stopped_work::IMPORT)
                && (std::time::Instant::now() < grace || import_busy(root).unwrap_or(false));
            if closed.load(std::sync::atomic::Ordering::Relaxed) || artifacts.is_err() || !importing {
                break;
            }
            trace("the import still copies: records and artifacts go round again".into());
            std::thread::sleep(Duration::from_secs(1));
        }
        if closed.load(std::sync::atomic::Ordering::Relaxed) {
            running.store(false, std::sync::atomic::Ordering::Relaxed);
            let _ = import.join();
            eprintln!("projekt je zatvoren: pozadinski proces staje");
            return Ok(false);
        }
        // Wave needs the audio facts the completion has just written; the filmstrips are
        // already made, so this pass only makes the waves the first one had to skip.
        let artifacts =
            artifacts.and_then(|()| run_artifacts(root, &lease, &closed, &std::sync::atomic::AtomicBool::new(false), &stops));
        running.store(false, std::sync::atomic::Ordering::Relaxed);
        let import = import.join().map_err(|_| qnc_ui_text::text("worker.import_crashed").to_string())?;
        // Once more after the import: a clip imported before its record was final could not
        // get its poster at Uvezi (user 2026-10-18).
        if !closed.load(std::sync::atomic::Ordering::Relaxed) {
            make_posters(root, &config, &lease, &stops);
            // The shots after the filmstrips and waves (user 2026-10-27: Video AI, a job of
            // its own, lower than the player).
            make_shots(root, &config, &lease, &stops, Look::All);
        }
        artifacts.and(import).map(|()| true)
    })
}

/// Posters of the clips without one, by their block (`qnc-missing-posters`); where each
/// medium is on this computer comes from the source bindings Select uses.
fn make_posters(
    root: &Path,
    config: &qnc_ingest_select::selection_config::SelectionConfig,
    lease: &qnc_playback_activity::Lease,
    stops: &Stops,
) {
    let cancel = stops.flag(qnc_stopped_work::POSTERS);
    if stops.is(qnc_stopped_work::POSTERS) {
        return;
    }
    let local_media = |uri: &str| {
        let reference = qnc_source_reader::SourceReference::from_uri(uri).ok()?;
        let source = config.sources.iter().find(|source| source.location.uri == reference.source_uri())?;
        source.local_media_path(uri).ok().flatten()
    };
    let stream_media = |uri: &str| qnc_ingest_select::selection_config::stream_of(&config.sources, uri);
    let project = match qnc_missing_posters::PosterProject::active(root) {
        Ok(project) => project,
        Err(error) => return eprintln!("posteri nisu napravljeni: {error}"),
    };
    match qnc_missing_posters::make_missing_posters(&project, &local_media, &stream_media, Some(lease.write_lease()), cancel) {
        Ok(0) => {}
        Ok(made) => eprintln!("posteri napravljeni: {made}"),
        Err(error) => eprintln!("posteri nisu napravljeni: {error}"),
    }
}

/// Whether the active project in the database is still the one a run started for.
fn still_active(active_project: &qnc_active_project_read::ActiveProjectReader, started: &str) -> bool {
    active_project
        .read()
        .is_ok_and(|snapshot| snapshot.settings.project_id == started)
}

/// Filmstrips and waves of the active project. While `completing`, records still become
/// final: the waves of newly final clips are taken every few seconds and the filmstrip waits.
fn run_artifacts(
    root: &Path,
    lease: &qnc_playback_activity::Lease,
    closed: &std::sync::atomic::AtomicBool,
    completing: &std::sync::atomic::AtomicBool,
    stops: &Stops,
) -> Result<(), String> {
    let active_project = qnc_active_project_read::ActiveProjectReader::from_root(root)
        .map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    let reader = active_project.settings_reader().clone();
    // A filmstrip or wave the user stopped is made as one the settings leave off; this changes
    // only what this run makes, the project's settings stay as they are.
    let stopped_now = || (stops.is(qnc_stopped_work::FILMSTRIPS), stops.is(qnc_stopped_work::WAVES));
    let without = |(filmstrips, waves): (bool, bool)| {
        let mut settings = snapshot.settings.clone();
        if let serde_json::Value::Object(artifacts) = &mut settings.artifacts {
            for (stopped, key) in [(filmstrips, "filmstrip"), (waves, "wave")] {
                if stopped {
                    artifacts.insert(key.into(), serde_json::Value::String("off".into()));
                }
            }
        }
        settings
    };
    let mut stopped = stopped_now();
    if stopped == (true, true) {
        return Ok(());
    }
    let mut settings = without(stopped);
    let target = qnc_content_store::ContentTarget::for_project(&reader, &settings)?;
    // Any player of any form announces itself in the project database; the
    // generators give way while it prepares or plays.
    // While the user edits two workers, while the player plays only the open clip's (user
    // 2026-10-20: measured, one core beside a playing player costs nothing).
    let level = qnc_playback_activity::playback_level(target.clone());
    // A transcription the user asked for counts as playing (user 2026-10-28: it crawled behind
    // the waves and filmstrips of a fresh import, read 157 s and engine 156 s for 254 s of
    // sound): only the open clip's artifacts go on beside it, the rest wait until it is done.
    let transcribing = transcription_running(target.clone());
    let player_works = || level() == qnc_playback_activity::PlaybackLevel::Playing || transcribing();
    let cap = || if transcribing() { qnc_playback_activity::PlaybackLevel::Playing.background_cap() } else { level().background_cap() };
    let start = |settings: &qnc_work_settings::WorkSettings| -> Result<qnc_content_artifacts::ProjectArtifacts, String> {
        let mut artifacts = qnc_content_artifacts::ProjectArtifacts::new();
        artifacts.set_host_root(root);
        // Published under the lease, checked by the database owner in the same transaction.
        artifacts.set_write_lease(lease.write_lease());
        artifacts.set_playback_priority(player_works());
        artifacts.set_cap(cap());
        artifacts.sync(&reader, settings, target.clone(), false)?;
        Ok(artifacts)
    };
    let mut artifacts = start(&settings)?;
    let deadline = std::time::Instant::now() + Duration::from_secs(60 * 60);
    let mut shown_checked = std::time::Instant::now() - Duration::from_secs(1);
    let mut synced = std::time::Instant::now();
    loop {
        let still_completing = completing.load(std::sync::atomic::Ordering::Relaxed);
        if !still_completing && !artifacts.has_pending_work() {
            break;
        }
        if closed.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(()); // dropping the artifacts cancels their workers
        }
        // The user stopped the filmstrips or the waves: the workers of both are cancelled (by
        // dropping them) and the ones still wanted start again without the stopped kind.
        if stopped_now() != stopped {
            stopped = stopped_now();
            drop(artifacts);
            if stopped == (true, true) {
                return Ok(());
            }
            settings = without(stopped);
            artifacts = start(&settings)?;
            continue;
        }
        // The lists are read again every 3 s while records are completed: a clip whose
        // record became final joins the waves.
        if still_completing && synced.elapsed() >= Duration::from_secs(3) {
            synced = std::time::Instant::now();
            artifacts.sync(&reader, &settings, target.clone(), false)?;
        }
        // The clip a preview has open (written to the project database) goes first.
        if shown_checked.elapsed() >= Duration::from_millis(500) {
            shown_checked = std::time::Instant::now();
            if let Some(clip_id) = qnc_playback_activity::read(&target, qnc_playback_activity::PLAYBACK_CLIP) {
                artifacts.prefer(&clip_id);
            }
        }
        artifacts.set_playback_priority(player_works());
        artifacts.set_cap(cap());
        let polled = artifacts.poll(None);
        if let Some(error) = polled.error {
            return Err(error);
        }
        if std::time::Instant::now() > deadline {
            return Err(qnc_ui_text::text("worker.artifacts_timeout").into());
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

/// Probes at once. The working media is USB 3 or faster (user 2026-10-18; USB 2.0 is no
/// working medium): there a probe is bound by the processor, and measured on a stick 8 at
/// once took 0.22 s a clip, 2 at once 0.37 s. The records are done before the artifacts
/// read the card, so the probes have it to themselves.
const PROBE_WORKERS: usize = 6;

fn root_argument() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--root" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

/// Shot analysis of the clips not analysed yet, by its block (`qnc-shot-analysis`); it waits
/// while the player plays and takes the clip a preview has open first.
fn make_shots(
    root: &Path,
    config: &qnc_ingest_select::selection_config::SelectionConfig,
    lease: &qnc_playback_activity::Lease,
    stops: &Stops,
    look: Look,
) {
    let result = (|| {
        let project = qnc_shot_analysis::ShotProject::active(root)?;
        let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
        let snapshot = active.read().map_err(|e| e.to_string())?;
        let target = qnc_content_store::ContentTarget::for_project(active.settings_reader(), &snapshot.settings)?;
        // When the template copies the clips the AI reads the copies, never the card (user
        // 2026-11-12: "ostale radnje idu iz kopiranih kadrova"): a clip not copied yet waits.
        let copying = snapshot.settings.ingest_copies();
        let copies: std::collections::HashMap<String, PathBuf> = match active.settings_reader().local_workspace_dir(&snapshot.settings).map_err(|e| e.to_string())? {
            Some(dir) if copying => qnc_record_completion::copies_of(&target, &dir)?.into_values().flatten().collect(),
            _ => Default::default(),
        };
        let local_media = |uri: &str| {
            if let Some(file) = copies.get(uri) {
                return Some(file.clone());
            }
            if copying {
                return None;
            }
            let reference = qnc_source_reader::SourceReference::from_uri(uri).ok()?;
            let source = config.sources.iter().find(|source| source.location.uri == reference.source_uri())?;
            source.local_media_path(uri).ok().flatten()
        };
        let level = qnc_playback_activity::playback_level(target.clone());
        // It also waits while a transcription the user asked for runs: both read the same card
        // (live 2026-10-27: the transcription read crawled behind the shot analysis).
        let transcribing = || qnc_playback_activity::is_fresh(&target, qnc_transcribe_queue::LEASE, qnc_playback_activity::WORKER_FRESH_SECONDS);
        let player_plays = || level() == qnc_playback_activity::PlaybackLevel::Playing || transcribing();
        let wanted = || qnc_playback_activity::read(&target, qnc_playback_activity::PLAYBACK_CLIP);
        // Each step its own pace: its cancel is its X (the project closed sets them all).
        let pace = |function| qnc_shot_analysis::Pace { player_plays: &player_plays, wanted: &wanted, cancel: stops.flag(function) };
        let (shots, content, captions) = (qnc_stopped_work::SHOTS, qnc_stopped_work::CONTENT, qnc_stopped_work::CAPTIONS);
        // First the quickest sorting, from each clip's poster (user 2026-11-10), before any
        // picture of a shot is read.
        if look != Look::Captions && !stops.is(content) {
            if let Err(error) = qnc_shot_content_run::look_posters(active.settings_reader(), &snapshot.settings, &local_media, Some(lease.write_lease()), &pace(content)) {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("shot-content posters failed error={error}"));
            }
        }
        if look == Look::Posters {
            return Ok(0);
        }
        let analysed = if look == Look::Captions || stops.is(shots) { Ok(0) } else { qnc_shot_analysis::analyse_missing(&project, &local_media, Some(lease.write_lease()), &pace(shots)) };
        // What the usable parts show (user 2026-11-10: sorting the B-roll by content), right
        // after their analysis, with the same pace.
        if look != Look::Captions && !stops.is(content) {
            if let Err(error) = qnc_shot_content_run::look_missing(active.settings_reader(), &snapshot.settings, &local_media, Some(lease.write_lease()), &pace(content)) {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("shot-content failed error={error}"));
            }
        }
        // A sentence of what each looked-at shot shows, from its poster (user 2026-11-10:
        // Florence-2 as the second AI engine), kept on this computer.
        if matches!(look, Look::Captions | Look::All) && !stops.is(captions) {
            if let Err(error) = qnc_shot_caption_run::caption_missing(root, active.settings_reader(), &snapshot.settings, &local_media, &pace(captions)) {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("shot-caption failed error={error}"));
            }
        }
        analysed
    })();
    match result {
        Ok(0) => {}
        Ok(made) => eprintln!("kadrovi analizirani: {made}"),
        Err(error) => {
            eprintln!("analiza kadrova nije uspjela: {error}");
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("shot-analysis failed error={error}"));
        }
    }
}

/// Whether a transcription the user asked for runs (its lease is fresh), read at most twice a second.
fn transcription_running(target: qnc_content_store::ContentTarget) -> impl Fn() -> bool {
    let last = std::cell::Cell::new(None::<(std::time::Instant, bool)>);
    move || {
        if let Some((at, value)) = last.get() {
            if at.elapsed() < Duration::from_millis(500) {
                return value;
            }
        }
        let value = qnc_playback_activity::is_fresh(&target, qnc_transcribe_queue::LEASE, qnc_playback_activity::WORKER_FRESH_SECONDS);
        last.set(Some((std::time::Instant::now(), value)));
        value
    }
}

/// The functions the user stopped (the X beside a bar of the card ingest window, user
/// 2026-11-12), read from the project database by the watch of a run once a second. A flag is
/// the cancel of its function: it stops what runs and nothing of it starts again; the run's
/// project closed sets every flag.
struct Stops([std::sync::atomic::AtomicBool; 7]);

impl Stops {
    fn new() -> Self {
        Self(std::array::from_fn(|_| std::sync::atomic::AtomicBool::new(false)))
    }

    fn flag(&self, function: &str) -> &std::sync::atomic::AtomicBool {
        let at = qnc_stopped_work::ALL.iter().position(|code| *code == function).expect("a function that can be stopped");
        &self.0[at]
    }

    fn is(&self, function: &str) -> bool {
        self.flag(function).load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The functions stopped in the database; a flag once set stays set for the run.
    fn read(&self, target: &qnc_content_store::ContentTarget) {
        for code in qnc_stopped_work::read(target) {
            if !self.is(&code) {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("worker stops {code} (the user)"));
                self.flag(&code).store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn all(&self) {
        for flag in &self.0 {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Whether the user stopped the import of the active project (its queued clips then wait for
/// nothing: the run does not go on for them).
fn import_stopped(
    active_project: &qnc_active_project_read::ActiveProjectReader,
    settings: &qnc_work_settings::WorkSettings,
) -> bool {
    qnc_content_store::ContentTarget::for_project(active_project.settings_reader(), settings)
        .is_ok_and(|target| qnc_stopped_work::read(&target).contains(qnc_stopped_work::IMPORT))
}

/// One line of where the worker is, in the log (live 2026-11-12: the copy ran, nothing else).
fn trace(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("worker {line}"));
}

/// What one turn of the AI does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Look {
    /// Only the first look, from the posters (right after Select: user 2026-11-10 "first the AI
    /// look at the card's posters").
    Posters,
    /// The posters, the analysis of the shots and what they show.
    Shots,
    /// Only the sentences about the shots (Florence-2, seconds a shot): on a thread of their own
    /// while the clips are copied, so the analysis never waits behind them (live 2026-11-12: the
    /// shots of 28 copied clips waited 2.5 min behind the captions).
    Captions,
    All,
}
