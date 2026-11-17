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
    // The conductor gives the steps after the records and posters their places, by priority and by
    // what it measures (user 2026-11-17: "trebaš tražiti drugačiju organizaciju procesa i
    // optimizirati dinamičkim prilagođavanjem"): the copy first, it feeds every other step; a lower
    // step gets a place only while the higher ones do not go slower (`qnc-ingest-conductor`).
    const CONDUCTED: [(Step, usize); 7] = [(Step::Import, 1), (Step::Transcripts, 1), (Step::Waves, 1), (Step::Filmstrips, 4), (Step::Shots, 1), (Step::Content, 1), (Step::Captions, 1)];
    let conducted: Vec<(Step, usize)> = CONDUCTED.iter().copied().filter(|(step, _)| steps.contains(step)).collect();
    let conductor = std::sync::Mutex::new(qnc_ingest_conductor::Conductor::new(conducted.iter().map(|(step, most)| {
        // The waves and the filmstrips never wait whole: one each always works beside the copy and the
        // transcription (user 2026-11-17: "filmstrip više od 6 minuta, neupotrebljivo").
        // The transcription keeps its place too: the journalist reads the statements first (user
        // 2026-11-17: "AI prijepis mora biti najbrži"; the log: it gave its place back to the noise
        // of the copy and waited 20 s for its first one).
        // The waves one clip at most (cheap, they ended long before the rest), the filmstrips two
        // always (user 2026-11-17: "smanji niti za val, povećaj za prijepis i filmstrip").
        let keeps = match step {
            Step::Transcripts | Step::Waves => 1,
            Step::Filmstrips => 2,
            _ => 0,
        };
        qnc_ingest_conductor::Step::new(step.code(), *most).keeping(keeps)
    }).collect())?);
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
        Step::Shots => editing_busy(),
        Step::Content => editing_busy() || not_done(Step::Shots),
        Step::Captions => editing_busy() || not_done(Step::Shots) || not_done(Step::Content),
        _ => false,
    };
    let may_start = |step: Step| match plan.stage_of(step) {
        Some(0) | None => true,
        Some(stage) => plan.stages[stage - 1].iter().all(|before| begun[at(*before)].load(Relaxed)),
    };
    let stuck_records = std::sync::Mutex::new(std::collections::BTreeSet::<String>::new());
    let failure = std::sync::Mutex::new(None::<String>);
    let result = std::thread::scope(|scope| {
        // The watch: the project closed, the user's X, the end of the run.
        scope.spawn(|| {
            let mut quiet = 0;
            let started_at = std::time::Instant::now();
            let mut processor = crate::Processor::new();
            // The copy and the speech heard, a second apart, over the last 10 s: the rates.
            let mut samples: std::collections::VecDeque<(f64, u64, f64)> = std::collections::VecDeque::new();
            while going.load(Relaxed) {
                {
                    let now = started_at.elapsed().as_secs_f64();
                    samples.push_back((now, qnc_ingest_import_worker::copied_bytes(), qnc_transcribe_run::heard_seconds()));
                    while samples.len() > 11 {
                        samples.pop_front();
                    }
                    let mut conductor = conductor.lock().unwrap_or_else(|e| e.into_inner());
                    for (step, _) in &conducted {
                        let i = at(*step);
                        let working = !begun[i].load(Relaxed) || !idle[i].load(Relaxed);
                        conductor.work(step.code(), usize::from(working), 0);
                    }
                    if let (Some(first), Some(last)) = (samples.front(), samples.back()) {
                        let span = last.0 - first.0;
                        if span >= 5.0 {
                            conductor.rate(Step::Import.code(), (last.1 - first.1) as f64 / 1e6 / span);
                            conductor.rate(Step::Transcripts.code(), (last.2 - first.2) / span);
                        }
                    }
                    if let Some(busy) = processor.busy() {
                        conductor.busy(busy);
                    }
                    for decision in conductor.tick(now) {
                        trace(format!("conductor {decision:?}"));
                    }
                    // The background transcription waits while it has no place.
                    if conducted_step(Step::Transcripts) {
                        qnc_transcribe_run::hold(conductor.places(Step::Transcripts.code()) == 0);
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
            let (may_start, places, ai_before_busy, make_backend, player_works, stuck) = (&may_start, &places, &ai_before_busy, &make_backend, &player_works, &stuck_records);
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
                        Step::PosterLook | Step::Shots | Step::Content | Step::Captions => crate::make_shots(root, config, lease, stops, step, &|| places(step) == 0 || ai_before_busy(step)) > 0,
                        Step::Import => match crate::import_waiting(root) {
                            // Working from the start of the round, not from its end (the conductor
                            // saw the copy idle while it copied: 2026-11-17 log).
                            Ok(true) => match crate::project(root).and_then(|project| {
                                idle.store(false, Relaxed);
                                qnc_ingest_import_worker::run_import(root, &project, run)
                            }) {
                                Ok(summary) => summary.imported + summary.failed > 0,
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
                            crate::transcribe::run(root, going, stops.flag(qnc_stopped_work::TRANSCRIPTS), idle);
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
    qnc_transcribe_run::hold(false);
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
