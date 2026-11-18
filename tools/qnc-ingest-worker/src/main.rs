//! Background application started by Select and by Uvezi. It reads the settings of the
//! active project once, takes the queued clips of the selection and does what the
//! settings say (link copies nothing, proxy or original copies), completes the card
//! records that playback still lacks something of (v5 media probe), builds the
//! artifacts, then ends. Its lease, the pause while a
//! player works and its result are kept in the project database, nowhere else.

use qnc_ingest_plan::Step;
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

mod run;
mod transcribe;

use run::run_once;

fn main() -> ExitCode {
    qnc_dev_diagnostics::log_panics("qnc-ingest-worker");
    // Below the transcription (user 2026-11-17: "AI prijepis (Parakeet) je prioritet, pa tek
    // onda val i onda filmstrip"): the waves, import and AI of this process give the processor
    // to the Parakeet program, which sets itself to normal; the player stays above both.
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, BELOW_NORMAL_PRIORITY_CLASS};
        SetPriorityClass(GetCurrentProcess(), BELOW_NORMAL_PRIORITY_CLASS);
    }
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
pub(crate) fn records_waiting(root: &Path) -> Result<Vec<String>, String> {
    let active_project = crate::project(root)
        .map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    qnc_content_read::ContentReader::for_project(active_project.settings_reader(), &snapshot.settings)?.records_waiting()
}

/// Whether clips wait in the import queue of the active project.
pub(crate) fn import_waiting(root: &Path) -> Result<bool, String> {
    let active_project = crate::project(root)
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
        // A clip left "processing" by a worker that ended (killed, crashed) is taken again by the
        // import (live 2026-11-12: the import never ran for it and the card was never finished).
        .any(|clip| matches!(clip.import_status.as_str(), "queued" | "processing")))
}

/// The sources of this computer now (a card attached after the worker started is among them,
/// live 2026-11-13); the ones read at the start when they cannot be read again.
pub(crate) fn sources_now(
    root: &Path,
    started: &qnc_ingest_select::selection_config::SelectionConfig,
) -> qnc_ingest_select::selection_config::SelectionConfig {
    qnc_ingest_select::selection_config::SelectionConfig::load(root).unwrap_or_else(|_| started.clone())
}

/// Posters of the clips without one, by their block (`qnc-missing-posters`); where each
/// medium is on this computer comes from the source bindings Select uses.
pub(crate) fn make_posters(
    root: &Path,
    config: &qnc_ingest_select::selection_config::SelectionConfig,
    lease: &qnc_playback_activity::Lease,
    stops: &Stops,
) -> usize {
    let cancel = stops.flag(qnc_stopped_work::POSTERS);
    if stops.is(qnc_stopped_work::POSTERS) {
        return 0;
    }
    let config = &sources_now(root, config);
    let local_media = |uri: &str| {
        let reference = qnc_source_reader::SourceReference::from_uri(uri).ok()?;
        let source = config.sources.iter().find(|source| source.location.uri == reference.source_uri())?;
        source.local_media_path(uri).ok().flatten()
    };
    let stream_media = |uri: &str| qnc_ingest_select::selection_config::stream_of(&config.sources, uri);
    let project = match crate::project(root).and_then(|active| active.read().map(|snapshot| qnc_missing_posters::PosterProject::new(active.settings_reader().clone(), snapshot.settings)).map_err(|e| e.to_string())) {
        Ok(project) => project,
        Err(error) => {
            eprintln!("posteri nisu napravljeni: {error}");
            return 0;
        }
    };
    match qnc_missing_posters::make_missing_posters(&project, &local_media, &stream_media, Some(lease.write_lease()), cancel) {
        Ok(0) => 0,
        Ok(made) => {
            eprintln!("posteri napravljeni: {made}");
            made
        }
        Err(error) => {
            eprintln!("posteri nisu napravljeni: {error}");
            0
        }
    }
}

/// Whether the active project in the database is still the one a run started for.
pub(crate) fn still_active(active_project: &qnc_active_project_read::ActiveProjectReader, started: &str) -> bool {
    active_project
        .read()
        .is_ok_and(|snapshot| snapshot.settings.project_id == started)
}

/// The filmstrips or the waves (`step`) of the run's project. While `going`, records still become
/// final: the list is read again every few seconds; `idle` says no clip waits for it now.
pub(crate) fn run_artifacts(
    root: &Path,
    lease: &qnc_playback_activity::Lease,
    closed: &std::sync::atomic::AtomicBool,
    going: &std::sync::atomic::AtomicBool,
    stops: &Stops,
    step: qnc_ingest_plan::Step,
    idle: &std::sync::atomic::AtomicBool,
    // How many clips this step may work on now (the conductor).
    places: &(dyn Fn() -> usize + Sync),
) -> Result<(), String> {
    let active_project = crate::project(root)
        .map_err(|error| error.to_string())?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    let reader = active_project.settings_reader().clone();
    // A filmstrip or wave the user stopped is made as one the settings leave off; this changes
    // only what this run makes, the project's settings stay as they are.
    // Only its own kind: the other is the step of its own thread.
    let stopped_now = || {
        (stops.is(qnc_stopped_work::FILMSTRIPS) || step != qnc_ingest_plan::Step::Filmstrips, stops.is(qnc_stopped_work::WAVES) || step != qnc_ingest_plan::Step::Waves)
    };
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
    // From the copies the transcription reads no card: the artifacts do not wait for it then.
    let copying = settings.ingest_copies();
    let transcription = transcription_running(target.clone());
    let transcribing = || !copying && transcription();
    // The places the conductor gives this step (user 2026-11-17: dynamic, by priority and by what
    // it measures): none, it waits (only the clip a preview has open goes on); else at most that many.
    let player_works = || level() == qnc_playback_activity::PlaybackLevel::Playing || transcribing() || places() == 0;
    let cap = || {
        let level_cap = if transcribing() { qnc_playback_activity::PlaybackLevel::Playing.background_cap() } else { level().background_cap() };
        Some(level_cap.map_or(places(), |cap| cap.min(places())).max(1))
    };
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
        let still_completing = going.load(std::sync::atomic::Ordering::Relaxed);
        idle.store(!artifacts.has_pending_work(), std::sync::atomic::Ordering::Relaxed);
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
pub(crate) const PROBE_WORKERS: usize = 6;

fn root_argument() -> Option<PathBuf> {
    argument("--root").map(PathBuf::from)
}

fn argument(name: &str) -> Option<std::ffi::OsString> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == name {
            return args.next();
        }
    }
    None
}

/// The project this run works in (user 2026-11-16: "gdje će se rezultati njihova rada snimati
/// određuju Orkestratori"): the one the orchestrator gives with `--project`; without it, the
/// project the database names (the active one, or the one pinned for this process).
pub(crate) fn project(root: &Path) -> Result<qnc_active_project_read::ActiveProjectReader, String> {
    match argument("--project").and_then(|id| id.into_string().ok()) {
        Some(id) => {
            let reader = qnc_work_settings::SettingsReader::from_root(root).map_err(|e| e.to_string())?.with_project(&id);
            Ok(qnc_active_project_read::ActiveProjectReader::from_settings_reader(reader))
        }
        None => qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|e| e.to_string()),
    }
}

/// Shot analysis of the clips not analysed yet, by its block (`qnc-shot-analysis`); it waits
/// while the player plays and takes the clip a preview has open first.
pub(crate) fn make_shots(
    root: &Path,
    config: &qnc_ingest_select::selection_config::SelectionConfig,
    lease: &qnc_playback_activity::Lease,
    stops: &Stops,
    step: Step,
    // The steps before are not done yet: the AI waits as for a playing player.
    earlier_busy: &(dyn Fn() -> bool + Sync),
) -> usize {
    let config = &sources_now(root, config);
    let result = (|| -> Result<usize, String> {
        let project = crate::project(root).and_then(|active| active.read().map(|snapshot| qnc_shot_analysis::ShotProject::new(active.settings_reader().clone(), snapshot.settings)).map_err(|e| e.to_string()))?;
        let active = crate::project(root).map_err(|e| e.to_string())?;
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
        // From the copies the AI shots and content wait for it too: the transcription has the
        // processor first (user 2026-11-17: "neka AI kadrovi i sadržaj čekaju dok prijepis ne
        // završi, prijepisu daj prednost"); the look from the posters does not wait.
        let waits_for_it = !copying || matches!(step, Step::Shots | Step::Content);
        let transcribing = || waits_for_it && qnc_playback_activity::is_fresh(&target, qnc_transcribe_queue::LEASE, qnc_playback_activity::WORKER_FRESH_SECONDS);
        let player_plays = || level() == qnc_playback_activity::PlaybackLevel::Playing || transcribing() || earlier_busy();
        let wanted = || qnc_playback_activity::read(&target, qnc_playback_activity::PLAYBACK_CLIP);
        // Each step its own pace: its cancel is its X (the project closed sets them all).
        // A clip whose first channel holds a statement is not looked at (user 2026-11-12); with
        // transcription on, a clip is taken once it is heard.
        let takes = transcribe::ai_takes(root, stops.is(qnc_stopped_work::TRANSCRIPTS))?;
        let pace = |function| {
            let takes: &dyn Fn(&str) -> bool = if function == qnc_stopped_work::SHOTS { &*takes.shots } else { &*takes.content };
            qnc_shot_analysis::Pace { player_plays: &player_plays, wanted: &wanted, takes, cancel: stops.flag(function) }
        };
        let (reader, settings, lease) = (active.settings_reader(), &snapshot.settings, Some(lease.write_lease()));
        // Only the step of the plan this turn is for (the order is the template's).
        Ok(match step {
            // The quickest sorting, from each clip's poster (user 2026-11-10), before any picture
            // of a shot is read.
            Step::PosterLook => qnc_shot_content_run::look_posters(reader, settings, &local_media, lease, &pace(qnc_stopped_work::CONTENT))?,
            Step::Shots => qnc_shot_analysis::analyse_missing(&project, &local_media, lease, &pace(qnc_stopped_work::SHOTS))?,
            // What the usable parts show (user 2026-11-10: sorting the B-roll by content).
            Step::Content => qnc_shot_content_run::look_missing(reader, settings, &local_media, lease, &pace(qnc_stopped_work::CONTENT))?,
            // A sentence of what each looked-at shot shows, from its poster (user 2026-11-10:
            // Florence-2 as the second AI engine), kept on this computer.
            Step::Captions => qnc_shot_caption_run::caption_missing(root, reader, settings, &local_media, &pace(qnc_stopped_work::CAPTIONS))?,
            _ => 0,
        })
    })();
    match result {
        Ok(made) => made,
        Err(error) => {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("{} failed error={error}", step.code()));
            0
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
pub(crate) struct Stops([std::sync::atomic::AtomicBool; qnc_stopped_work::ALL.len()]);

impl Stops {
    fn new() -> Self {
        Self(std::array::from_fn(|_| std::sync::atomic::AtomicBool::new(false)))
    }

    pub(crate) fn flag(&self, function: &str) -> &std::sync::atomic::AtomicBool {
        let at = qnc_stopped_work::ALL.iter().position(|code| *code == function).expect("a function that can be stopped");
        &self.0[at]
    }

    pub(crate) fn is(&self, function: &str) -> bool {
        self.flag(function).load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The functions stopped in the database; a flag once set stays set for the run.
    pub(crate) fn read(&self, target: &qnc_content_store::ContentTarget) {
        for code in qnc_stopped_work::read(target) {
            if !self.is(&code) {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("worker stops {code} (the user)"));
                self.flag(&code).store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    pub(crate) fn all(&self) {
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
pub(crate) fn trace(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("worker {line}"));
}

/// The project worked on is a card base: what its shot analysis says it holds is written beside
/// the user's description (`qnc-base-contents`); a story's project is left alone.
pub(crate) fn describe_base(root: &std::path::Path, reader: &qnc_work_settings::SettingsReader, settings: &qnc_work_settings::WorkSettings) {
    let log = |line: String| qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
    let is_base = qnc_ingest_base::all(root).is_ok_and(|bases| bases.iter().any(|base| base.id == settings.project_id));
    if !is_base {
        return;
    }
    let written = qnc_db_broker::ProjectDbTarget::for_project(reader, settings)
        .and_then(|target| qnc_base_contents::describe(root, &target, target.uri()))
        .and_then(|text| qnc_ingest_base::set_analysis(root, &settings.project_id, &text).map(|()| text.len()));
    match written {
        Ok(bytes) => log(format!("base-contents {} written ({bytes} bytes)", settings.project_id)),
        Err(error) => log(format!("base-contents {} not written: {error}", settings.project_id)),
    }
}

/// The share of the processor busy, from one reading to the next (the conductor's measure).
pub(crate) struct Processor {
    last: Option<(u64, u64)>,
}

impl Processor {
    pub(crate) fn new() -> Self {
        Self { last: None }
    }

    /// The share busy since the last call (0..1); none the first time or where it is not read.
    pub(crate) fn busy(&mut self) -> Option<f64> {
        let (idle, total) = times()?;
        let share = self.last.and_then(|(was_idle, was_total)| {
            let span = total.checked_sub(was_total).filter(|span| *span > 0)?;
            Some(1.0 - idle.saturating_sub(was_idle) as f64 / span as f64)
        });
        self.last = Some((idle, total));
        share
    }
}

/// The idle and the whole time of every processor (100 ns units).
#[cfg(windows)]
fn times() -> Option<(u64, u64)> {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::GetSystemTimes;
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut idle, mut kernel, mut user) = (zero, zero, zero);
    // SAFETY: three valid out pointers.
    if unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) } == 0 {
        return None;
    }
    let value = |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
    // The kernel time includes the idle time.
    Some((value(idle), value(kernel) + value(user)))
}

/// Not read on other systems: the conductor goes by the rates alone.
#[cfg(not(windows))]
fn times() -> Option<(u64, u64)> {
    None
}

/// What each program of the ingest uses of the processor (user 2026-11-17: "i dalje ne mjeriš"):
/// the processor time of every process by its name, between two calls, in cores; and the job's,
/// which counts the programs that started and ended between two calls too.
pub(crate) struct Programs {
    last: std::collections::HashMap<u32, (String, u64)>,
    at: std::time::Instant,
    job: Option<u64>,
    job_cores: Option<f64>,
}

/// The ingest's own programs (their executables' names without the extension); every other
/// process is measured too, for the log.
pub(crate) const MEASURED: [&str; 7] = ["qnc-transcribe-parakeet", "ffmpeg", "qnc-decoder-ffmpeg", "qnc-ingest-worker", "qnc-ai-card-ingest", "qnc-shot-content-siglip", "qnc-shot-caption-florence"];

impl Programs {
    pub(crate) fn new() -> Self {
        Self { last: Default::default(), at: std::time::Instant::now(), job: qnc_process_job::Job::own_processor_time(), job_cores: None }
    }

    /// Cores each measured program used since the last call, by name (the first call: none).
    pub(crate) fn used(&mut self) -> Vec<(String, f64)> {
        let now = process_times();
        let span = self.at.elapsed().as_secs_f64();
        self.at = std::time::Instant::now();
        let job = qnc_process_job::Job::own_processor_time();
        self.job_cores = job.zip(self.job).map(|(now, was)| now.saturating_sub(was) as f64 / 1e7 / span.max(0.001));
        self.job = job;
        let mut used: std::collections::BTreeMap<String, f64> = Default::default();
        for (pid, (name, time)) in &now {
            if let Some((_, was)) = self.last.get(pid) {
                *used.entry(name.clone()).or_default() += time.saturating_sub(*was) as f64 / 1e7 / span.max(0.001);
            }
        }
        let first = self.last.is_empty();
        self.last = now;
        if first {
            return Vec::new();
        }
        used.into_iter().collect()
    }

    /// Cores every process of the job used over the last `used` span (ended ones too).
    pub(crate) fn job_cores(&self) -> Option<f64> {
        self.job_cores
    }
}

/// Each measured process: its name and its processor time so far (100 ns units).
#[cfg(windows)]
fn process_times() -> std::collections::HashMap<u32, (String, u64)> {
    use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
    let mut times = std::collections::HashMap::new();
    // SAFETY: a snapshot this function owns and closes; the entry is sized; each handle is closed.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return times;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let length = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            let exe = String::from_utf16_lossy(&entry.szExeFile[..length]);
            let name = exe.strip_suffix(".exe").unwrap_or(&exe).to_string();
            {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
                if !process.is_null() {
                    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
                    let (mut created, mut ended, mut kernel, mut user) = (zero, zero, zero, zero);
                    if GetProcessTimes(process, &mut created, &mut ended, &mut kernel, &mut user) != 0 {
                        let value = |time: FILETIME| (u64::from(time.dwHighDateTime) << 32) | u64::from(time.dwLowDateTime);
                        times.insert(entry.th32ProcessID, (name, value(kernel) + value(user)));
                    }
                    CloseHandle(process);
                }
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    times
}

#[cfg(not(windows))]
fn process_times() -> std::collections::HashMap<u32, (String, u64)> {
    Default::default()
}
