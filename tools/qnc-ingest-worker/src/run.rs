//! One run of the worker: the plan of the project's template carried out (user 2026-11-16:
//! "redosljed bi trebao moći biti variabilan i trebao bi ući u postavke templatea"; the order
//! is `artifacts.order`, `qnc-ingest-plan`). The blocks only do their work; this says in which
//! project (the one the orchestrator gave, `crate::project`) and in which order.
//!
//! A stage starts once every step of the stage before it has done its first round (user
//! 2026-11-16, way "a": a stage starts when the one before gives its first results, not when it
//! is done): the clips copied first are worked on while the card is still copied. Every step
//! goes round, each time taking what is there for it; the run ends when every step found
//! nothing to do, twice in a row.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;

use qnc_ingest_plan::{Plan, Step};

use crate::{trace, Stops};

/// The stop flag of a step (the X beside its bar); the records have none.
fn stop_code(step: Step) -> Option<&'static str> {
    Some(match step {
        Step::Records => return None,
        Step::Posters => qnc_stopped_work::POSTERS,
        Step::PosterLook | Step::Content => qnc_stopped_work::CONTENT,
        Step::Waves => qnc_stopped_work::WAVES,
        Step::Filmstrips => qnc_stopped_work::FILMSTRIPS,
        Step::Import => qnc_stopped_work::IMPORT,
        Step::Transcripts => qnc_stopped_work::TRANSCRIPTS,
        Step::Shots => qnc_stopped_work::SHOTS,
        Step::Captions => qnc_stopped_work::CAPTIONS,
    })
}

/// How long a step waits between two rounds.
const ROUND: Duration = Duration::from_secs(1);

/// One run under the lease; false when another worker holds it.
pub(crate) fn run_once(root: &Path) -> Result<bool, String> {
    let active_project = crate::project(root)?;
    let snapshot = active_project.read().map_err(|error| error.to_string())?;
    let started = snapshot.settings.project_id.clone();
    // The order is the template's: a project without it is a controlled error, never a default.
    let plan = Plan::of_settings(&snapshot.settings.artifacts)?;
    let reader = active_project.settings_reader().clone();
    let target = qnc_content_store::ContentTarget::for_project(&reader, &snapshot.settings)?;
    let player_works = qnc_playback_activity::playback_plays(target.clone());
    let stop_target = target.clone();
    // The transcription queue of the base: its clips heard whole (done or failed) are the transcription's count.
    let transcripts = qnc_db_broker::ProjectDbTarget::for_project(active_project.settings_reader(), &snapshot.settings);
    let stops = Stops::new();
    stops.read(&stop_target);
    // Taken in one statement with its owner (external audit 2026-10-17): two starts never
    // both run; a run whose lease another took stops.
    let Some(lease) =
        qnc_playback_activity::Lease::claim(target, qnc_playback_activity::WORKER, qnc_playback_activity::WORKER_FRESH_SECONDS)?
    else {
        return Ok(false);
    };
    trace(format!("plan {}", plan.line()));
    let config = qnc_ingest_select::selection_config::SelectionConfig::load(root).map_err(|error| error.to_string())?;
    let make_backend = |source_uri: &str, media: &[qnc_source_reader::SourceReference], copies: &[(String, PathBuf)]| {
        // A card attached after the start is not among the sources read then: read them again.
        let fresh;
        let config = if config.sources.iter().any(|source| source.location.uri == source_uri) {
            &config
        } else {
            fresh = crate::sources_now(root, &config);
            &fresh
        };
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
    // The project closed (or another one active, or its base deleted): the run stops and lets
    // its database go.
    let closed = AtomicBool::new(false);
    let cancel = AtomicBool::new(false);
    // The steps go round while this is set; it is cleared once every step is idle.
    let going = AtomicBool::new(true);
    let run = qnc_ingest_import_worker::ImportRun { project_id: &started, lease: &lease, cancel: stops.flag(qnc_stopped_work::IMPORT) };
    let steps: Vec<Step> = plan.stages.iter().flatten().copied().collect();
    let begun: Vec<AtomicBool> = steps.iter().map(|_| AtomicBool::new(false)).collect();
    // Whether the step's last round found nothing to do.
    let idle: Vec<AtomicBool> = steps.iter().map(|_| AtomicBool::new(false)).collect();
    // How many steps are in a round of work now (the watch ends the run only at none).
    let rounds = std::sync::Mutex::new(0_usize);
    let at = |step: Step| steps.iter().position(|other| *other == step).expect("a step of the plan");
    // The conductor (`qnc-ingest-conductor`) keeps the clips done by the transcription, the waves
    // and the filmstrips about level (user 2026-11-17: "cilj je da AI prijepis, val i filmstrip
    // imaju približno isti broj obrađenih klipova"; "bitno je da svi završe približno u isto
    // vrijeme"): each step's share of its work done is kept level, the one behind gets the
    // processor's threads. The copy is not conducted: it reads the card and feeds them all.
    let cores = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    // The transcription's places are its programs (the AI catalog: how many and their threads).
    // With the graphics card's engine too, its program is one place more (it costs the processor
    // little; a place costs the threads of the processor's programs).
    let (programs, program_threads) = qnc_ai_catalog::installed_fast_transcribers().map_or((1, 4), |engines| {
        let programs = engines.iter().map(|engine| engine.parallel.max(1) as usize).sum();
        let threads = engines.iter().filter(|engine| engine.backend == "cpu").map(|engine| engine.threads.max(1) as usize).max().unwrap_or(1);
        (programs, threads)
    });
    let conducted: Vec<(Step, qnc_ingest_conductor::Step)> = [
        // The transcription holds all its places while it has work, from the first second (user
        // 2026-11-18: during the copy it got one or two of three, kept level by clips done while its
        // long clips were left for the end); the filmstrips keep one; the fast waves may wait.
        qnc_ingest_conductor::Step::new(Step::Transcripts.code(), programs).keeping(programs).costing(program_threads).balanced(),
        qnc_ingest_conductor::Step::new(Step::Waves.code(), 4).balanced(),
        qnc_ingest_conductor::Step::new(Step::Filmstrips.code(), cores).keeping(1).balanced(),
        qnc_ingest_conductor::Step::new(Step::Shots.code(), 1),
        qnc_ingest_conductor::Step::new(Step::Content.code(), 1),
        qnc_ingest_conductor::Step::new(Step::Captions.code(), 1),
    ]
    .into_iter()
    .filter_map(|spec| steps.iter().copied().find(|step| step.code() == spec.code).map(|step| (step, spec)))
    .collect();
    let conductor = std::sync::Mutex::new(qnc_ingest_conductor::Conductor::new(conducted.iter().map(|(_, spec)| spec.clone()).collect(), cores)?);
    let conducted_step = |step: Step| conducted.iter().any(|(other, _)| *other == step);
    // How many clips a conducted step may work on now (a step the conductor does not lead: always).
    let places = |step: Step| if conducted_step(step) { conductor.lock().unwrap_or_else(|e| e.into_inner()).places(step.code()) } else { usize::MAX };
    // SigLIP after the AI shots, the captions after both (user 2026-11-17): an order of their
    // results, not of the processor.
    let not_done = |step: Step| steps.iter().position(|other| *other == step).is_some_and(|i| !(begun[i].load(Relaxed) && idle[i].load(Relaxed)));
    // The AI only after the copy, the transcription, the waves and the filmstrips (user
    // 2026-11-17: "AI kadrovi tek kad je to gotovo, a nakon njega SigLIP"; the log: the shots got a
    // place by a probe while the card was being copied).
    let editing_busy = || [Step::Import, Step::Transcripts, Step::Waves, Step::Filmstrips].into_iter().any(not_done);
    let ai_before_busy = |step: Step| match step {
        // SigLIP's look at the card's posters too (2026-11-17 measurement: 0.3-0.6 of a core the
        // whole copy long).
        Step::PosterLook | Step::Shots => editing_busy(),
        Step::Content => editing_busy() || not_done(Step::Shots),
        Step::Captions => editing_busy() || not_done(Step::Shots) || not_done(Step::Content),
        _ => false,
    };
    // Breaking news on this computer (the tray's setting; user 2026-11-18): the waves first, every
    // AI step waits while they have work ("bitno je da val bude što prije gotov"); the filmstrips
    // beside them; then the AI shots, SigLIP, the captions; each clip by clip (below). Another mode
    // keeps the order below.
    let breaking = matches!(qnc_tray_mode::read(root), Ok(Some(qnc_tray_mode::Mode::Local { work: qnc_tray_mode::Work::BreakingNews })));
    trace(format!("order {}", if breaking { "breaking_news" } else { "level" }));
    // Clip by clip, as QNC-pro does (user 2026-11-22: "pravilo očito nije bilo dobro, koristi
    // system koji koristi qnc-pro, to je brže"; live: no filmstrip for the whole copy of a card):
    // every step reads the copies, so the waves and the filmstrips take each clip as soon as it is
    // copied, never waiting for the whole card; the AI shots take the clips whose wave is made
    // while the waves have nothing to do, SigLIP after the shots, the captions last.
    let breaking_held = |step: Step| {
        let waves = not_done(Step::Waves);
        match step {
            Step::Waves | Step::Filmstrips => false,
            Step::PosterLook | Step::Shots => waves,
            Step::Content => waves || not_done(Step::Shots),
            // The statements are heard with the waves, from their sound (user 2026-11-19); the queue
            // takes the clips they did not decide once the waves are done.
            Step::Transcripts => false,
            Step::Captions => waves || not_done(Step::Shots) || not_done(Step::Content),
            _ => false,
        }
    };
    // While a clip plays all the background waits (user 2026-11-18: "pozadinske zaustavi dok radi
    // play"): the conductor gives no place, the transcription and the AI hold; the copy and the
    // records do not wait (their own rules).
    let level = qnc_playback_activity::playback_level(stop_target.clone());
    let playing = || level() == qnc_playback_activity::PlaybackLevel::Playing;
    let held = |step: Step| playing() || if breaking { breaking_held(step) } else { ai_before_busy(step) };
    let may_start = |step: Step| match plan.stage_of(step) {
        Some(0) | None => true,
        Some(stage) => plan.stages[stage - 1].iter().all(|before| begun[at(*before)].load(Relaxed)),
    };
    let hear_programs = qnc_ai_catalog::installed_fast_transcribers().map_or(1, |engines| engines.iter().map(|engine| engine.parallel.max(1) as usize).sum::<usize>()).max(1);
    let stuck_records = std::sync::Mutex::new(std::collections::BTreeSet::<String>::new());
    let failure = std::sync::Mutex::new(None::<String>);
    let result = std::thread::scope(|scope| {
        // The watch: the project closed, the user's X, the end of the run.
        scope.spawn(|| {
            let mut quiet = 0;
            let started_at = std::time::Instant::now();
            let mut processor = crate::Processor::new();
            let mut programs = crate::Programs::new();
            let mut programs_at = std::time::Instant::now();
            let mut read_at: Option<std::time::Instant> = None;
            let mut counted_at: Option<std::time::Instant> = None;
            // What the base said last, and the counts of this process then: between two reads
            // of the base each step's +1 for every clip it finished (`qnc-done-count`; user
            // 2026-11-17: "ne brojati, treba aplikacija samo poslati u array za svaki odrađeni
            // +1"). The base is read again every 30 s and then is the truth.
            let counted = [qnc_done_count::IMPORTED, qnc_done_count::TRANSCRIPTS, qnc_done_count::WAVES, qnc_done_count::FILMSTRIPS];
            let counts_now = || counted.map(qnc_done_count::count);
            let mut base: Option<(qnc_work_progress::Progress, usize, [usize; 4])> = None;
            while going.load(Relaxed) {
                {
                    let now = started_at.elapsed().as_secs_f64();
                    let again = if base.is_some() { Duration::from_secs(30) } else { Duration::from_secs(2) };
                    if read_at.is_none_or(|at| at.elapsed() >= again) {
                        read_at = Some(std::time::Instant::now());
                        let counts = counts_now();
                        // A clip heard: every piece of it done or failed.
                        let heard = transcripts.as_ref().ok().and_then(|target| qnc_transcript_db::read_queue(target).ok()).map_or(0, |queue| {
                            let clips: std::collections::BTreeSet<&str> = queue.iter().map(|entry| entry.clip_id.as_str()).collect();
                            let open: std::collections::BTreeSet<&str> = queue.iter().filter(|entry| matches!(entry.state, qnc_transcript_db::QueueState::Waiting | qnc_transcript_db::QueueState::Running)).map(|entry| entry.clip_id.as_str()).collect();
                            clips.difference(&open).count()
                        });
                        if let Ok(progress) = qnc_work_progress::progress_of(root, &started) {
                            base = Some((progress, heard, counts));
                        }
                    }
                    // Each step's clips done of all the clips (every 2 s; user 2026-11-17: "broj
                    // klipova je bitan, ništa drugo"): the transcription's are the clips its queue
                    // heard whole (a part saved is not a clip done).
                    let progress = counted_at.is_none_or(|at| at.elapsed() >= Duration::from_secs(2)).then(|| {
                        counted_at = Some(std::time::Instant::now());
                        base.map(|(mut progress, heard, then)| {
                            let more = |i: usize| counts_now()[i].saturating_sub(then[i]);
                            let up = |done: usize, i: usize| (done + more(i)).min(progress.clips);
                            progress.imported = up(progress.imported, 0);
                            progress.waves = progress.waves.map(|done| up(done, 2));
                            progress.filmstrips = progress.filmstrips.map(|done| up(done, 3));
                            (progress, up(heard, 1))
                        })
                    });
                    let mut conductor = conductor.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(Some((progress, heard))) = progress {
                        // The transcription takes a block while its queue works.
                        let hearing = qnc_playback_activity::is_fresh(&stop_target, qnc_transcribe_queue::LEASE, qnc_playback_activity::WORKER_FRESH_SECONDS);
                        for (step, _) in &conducted {
                            let made = |count: Option<usize>| count.unwrap_or_default();
                            let clips = |count: Option<usize>| (made(count) as f64, progress.clips as f64, made(count) < progress.imported);
                            let (done, all, can) = match step {
                                Step::Transcripts => (heard as f64, progress.clips as f64, hearing),
                                Step::Waves => clips(progress.waves),
                                Step::Filmstrips => clips(progress.filmstrips),
                                Step::Shots => clips(progress.shots),
                                Step::Content => clips(progress.content),
                                Step::Captions => clips(progress.captions),
                                _ => (0.0, 0.0, false),
                            };
                            // A step the user stopped, or the AI waiting for the steps before it,
                            // takes no place.
                            let stopped = stop_code(*step).is_some_and(|code| stops.is(code));
                            // The waves and the filmstrips with nothing their workers can take hold no
                            // place: a clip whose record never became final is no work for them (the
                            // 2026-11-18 log: two clips whose probe failed held four places of the
                            // waves to the end).
                            let nothing = matches!(step, Step::Waves | Step::Filmstrips) && idle[at(*step)].load(Relaxed);
                            conductor.work(step.code(), done, all, can && !stopped && !nothing && !held(*step));
                        }
                    }
                    // The processor measured beside the decision: the log shows what the places did.
                    let busy = processor.busy();
                    // What each program uses of the processor, every 10 s.
                    if programs_at.elapsed() >= Duration::from_secs(10) {
                        programs_at = std::time::Instant::now();
                        let used = programs.used();
                        if !used.is_empty() {
                            // The ingest's programs, then the six biggest others and the sum of all
                            // (user 2026-11-18: the processor read full while QNC used half of it).
                            let ours: Vec<String> = used.iter().filter(|(name, _)| crate::MEASURED.contains(&name.as_str())).map(|(name, cores)| format!("{name}={cores:.2}")).collect();
                            let mut others: Vec<&(String, f64)> = used.iter().filter(|(name, cores)| !crate::MEASURED.contains(&name.as_str()) && *cores >= 0.05).collect();
                            others.sort_by(|a, b| b.1.total_cmp(&a.1));
                            let others: Vec<String> = others.iter().take(6).map(|(name, cores)| format!("{name}={cores:.2}")).collect();
                            let all: f64 = used.iter().map(|(_, cores)| cores).sum();
                            // The job counts every program of the ingest, the short ones ended between two looks too.
                            let job = programs.job_cores().map_or_else(|| "?".to_string(), |cores| format!("{cores:.2}"));
                            trace(format!("cpu cores {} | others {} | all={all:.2} job={job} of {cores}", ours.join(" "), others.join(" ")));
                        }
                    }
                    if let Some(decision) = conductor.tick(now) {
                        let busy = busy.map_or_else(|| "?".to_string(), |busy| format!("{:.0} %", busy * 100.0));
                        let steps: Vec<String> = decision.steps.iter().map(|(code, places, left, all)| format!("{code} left={left}/{all} places={places}")).collect();
                        // Why the AI waits (2026-11-19: the AI shots held while nothing seemed to play).
                        let why = format!("playing={} waves={} import={} shots={}", playing(), not_done(Step::Waves), not_done(Step::Import), not_done(Step::Shots));
                        trace(format!("conductor processor={busy} | {} | held {why}", steps.join(" | ")));
                    }
                    // The background transcription waits while it has no place.
                    if conducted_step(Step::Transcripts) {
                        // Never held by the count: a clip with part of its transcript saved counts as done while its
                        // queue still hears it (2026-11-17 test: the last two clips held for ever); the queue ends
                        // the transcription itself.
                        // In Breaking news it waits for the steps before it.
                        // At least as many as the fast engine has programs: the statements heard with the
                        // waves (one clip a program) take no place of the conductor's steps.
                        let places = if (breaking || playing()) && held(Step::Transcripts) { 0 } else { conductor.places(Step::Transcripts.code()).max(hear_programs) };
                        qnc_transcribe_run::set_places(places);
                    }
                }
                if !crate::still_active(&active_project, &started) || !lease.held() || qnc_ingest_base::closing(root, &started) {
                    closed.store(true, Relaxed);
                    cancel.store(true, Relaxed);
                    stops.all();
                    going.store(false, Relaxed);
                    return;
                }
                stops.read(&stop_target);
                // The run ends only when nothing is in a round: a round begun after the steps
                // looked idle (the import after Select) is watched to its end (2026-11-17 log:
                // the watch left, the copy went on and the user's stop was never read).
                {
                    let rounds = rounds.lock().unwrap_or_else(|e| e.into_inner());
                    let all_idle = *rounds == 0 && steps.iter().enumerate().all(|(i, _)| begun[i].load(Relaxed) && idle[i].load(Relaxed));
                    quiet = if all_idle { quiet + 1 } else { 0 };
                    if quiet >= 2 {
                        going.store(false, Relaxed);
                        return;
                    }
                }
                std::thread::sleep(ROUND);
            }
        });
        for (i, step) in steps.iter().copied().enumerate() {
            let rounds = &rounds;
            let (begun, idle, going, closed, cancel, stops, config, lease, run, failure) =
                (&begun[i], &idle[i], &going, &closed, &cancel, &stops, &config, &lease, &run, &failure);
            let (may_start, places, held, make_backend, player_works, stuck) = (&may_start, &places, &held, &make_backend, &player_works, &stuck_records);
            scope.spawn(move || {
                while !may_start(step) {
                    if !going.load(Relaxed) {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                let stopped = || stop_code(step).is_some_and(|code| stops.is(code));
                trace(format!("step {} starts", step.code()));
                // The look from the posters does not hold the stage after it (audit 2026-11-17: the
                // import and the transcription waited for its first round of SigLIP).
                if step == Step::PosterLook {
                    begun.store(true, Relaxed);
                }
                let mut first = true;
                loop {
                    if stopped() || closed.load(Relaxed) {
                        begun.store(true, Relaxed);
                        idle.store(true, Relaxed);
                        return;
                    }
                    // A round begins only while the run goes on, and the watch does not end the
                    // run while it lasts.
                    {
                        let mut rounds = rounds.lock().unwrap_or_else(|e| e.into_inner());
                        if !going.load(Relaxed) {
                            return;
                        }
                        *rounds += 1;
                    }
                    let in_round = InRound(rounds);
                    // The AI waits inside its round for a place; waiting it has work, else the
                    // conductor never gives it one (2026-11-17 log: after the filmstrips nothing led
                    // and the AI shots waited for ever).
                    if matches!(step, Step::Shots | Step::Content | Step::Captions) {
                        idle.store(false, Relaxed);
                    }
                    let did = match step {
                        Step::Records => {
                            let waiting = crate::records_waiting(root).unwrap_or_default();
                            let mut stuck = stuck.lock().expect("stuck records");
                            let fresh = waiting.iter().any(|clip| !stuck.contains(clip));
                            if fresh {
                                idle.store(false, Relaxed);
                            }
                            if first || fresh {
                                match crate::project(root).and_then(|project| {
                                    qnc_record_completion::complete_project(&project, make_backend, crate::PROBE_WORKERS, &**player_works, cancel, Some(lease.write_lease()))
                                }) {
                                    Ok(done) => {
                                        for (clip, error) in done.failed {
                                            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("record-completion failed clip={clip} error={error}"));
                                            stuck.insert(clip);
                                        }
                                        !done.completed.is_empty()
                                    }
                                    Err(error) => {
                                        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("record-completion failed error={error}"));
                                        false
                                    }
                                }
                            } else {
                                false
                            }
                        }
                        Step::Posters => crate::make_posters(root, config, lease, stops) > 0,
                        Step::PosterLook | Step::Shots | Step::Content | Step::Captions => crate::make_shots(root, config, lease, stops, step, &|| places(step) == 0 || held(step), breaking, &|| not_done(Step::Transcripts)) > 0,
                        // The copy waits for the posters (user 2026-11-22, a card on the network:
                        // the 207 posters, small files, waited about three minutes behind the copy
                        // of the originals on the same link; QNC-pro shows them first).
                        Step::Import if not_done(Step::Posters) => false,
                        Step::Import => match crate::import_waiting(root) {
                            // Working from the start of the round, not from its end (the conductor
                            // saw the copy idle while it copied: 2026-11-17 log).
                            Ok(true) => match crate::project(root).and_then(|project| {
                                idle.store(false, Relaxed);
                                qnc_ingest_import_worker::run_import(root, &project, run)
                            }) {
                                Ok(summary) => {
                                    // A project made from a base takes the base's statements once their clips are in it.
                                    if summary.imported > 0 {
                                        crate::copy_base_statements(root);
                                    }
                                    summary.imported + summary.failed > 0
                                }
                                Err(error) => {
                                    *failure.lock().expect("failure") = Some(error);
                                    false
                                }
                            },
                            _ => false,
                        },
                        // These keep their own service for the whole run.
                        Step::Waves | Step::Filmstrips => {
                            // A service for the whole run, not a round.
                            drop(in_round);
                            begun.store(true, Relaxed);
                            if let Err(error) = crate::run_artifacts(root, lease, closed, going, stops, step, idle, &|| places(step)) {
                                *failure.lock().expect("failure") = Some(error);
                            }
                            idle.store(true, Relaxed);
                            return;
                        }
                        Step::Transcripts => {
                            drop(in_round);
                            begun.store(true, Relaxed);
                            // The clips whose wave this run reads are heard from its sound (statements only); the
                            // queue takes the rest once the copy and the waves are done (live 2026-11-19: before
                            // anything was copied the waves looked done and a long clip was heard twice).
                            crate::transcribe::run(root, going, stops.flag(qnc_stopped_work::TRANSCRIPTS), idle, &|| !not_done(Step::Import) && !not_done(Step::Waves));
                            idle.store(true, Relaxed);
                            return;
                        }
                    };
                    drop(in_round);
                    first = false;
                    begun.store(true, Relaxed);
                    idle.store(!did, Relaxed);
                    if !going.load(Relaxed) {
                        return;
                    }
                    std::thread::sleep(ROUND);
                }
            });
        }
        Ok::<(), String>(())
    });
    qnc_transcribe_run::set_places(usize::MAX);
    result?;
    if closed.load(Relaxed) {
        eprintln!("projekt je zatvoren: pozadinski proces staje");
        return Ok(false);
    }
    crate::describe_base(root, &reader, &snapshot.settings);
    match failure.into_inner().expect("failure") {
        Some(error) => Err(error),
        None => Ok(true),
    }
}

/// A step in a round of work: the count goes down when the round ends, however it ends.
struct InRound<'a>(&'a std::sync::Mutex<usize>);

impl Drop for InRound<'_> {
    fn drop(&mut self) {
        let mut rounds = self.0.lock().unwrap_or_else(|e| e.into_inner());
        *rounds = rounds.saturating_sub(1);
    }
}
