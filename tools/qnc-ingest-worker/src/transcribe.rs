//! The transcription of the first channel during the ingest, and the rule it gives the AI shots
//! (user 2026-11-12: "rutina da se radi transkript na kadrovima koji sadrže prvi audio kanal";
//! "klip s prvim audio kanalom koji sadrži izjave ne analizira se sa AI kadrovi"; decided: the
//! words cover at least 30 % of the clip). Each clip once imported (copied, when the template
//! copies) goes into the project's transcription queue (`qnc-transcript-db`) as a whole, with the
//! fast engine and the speech channel of the project; the queue's runner (`qnc-transcribe-queue`)
//! hears it from the clip's medium (the copy, through the player input). A clip whose words cover
//! the share of a statement is not given to the AI shots.

use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

use qnc_transcript_db::{QueueQuality, QueueState, Queued};

/// The longest piece of a clip given to one transcription program.
const PIECE_SECONDS: f64 = 60.0;

/// The project of the active project's transcription, read once a run.
pub(crate) struct Transcription {
    db: qnc_db_broker::ProjectDbTarget,
    locator: qnc_still_locator::StillLocator,
    content: qnc_content_read::ContentReader,
}

impl Transcription {
    /// The transcription of the active project; `None` when its AI has no transcription.
    pub(crate) fn of_active(root: &Path) -> Result<Option<Self>, String> {
        let active = crate::project(root).map_err(|e| e.to_string())?;
        let snapshot = active.read().map_err(|e| e.to_string())?;
        if !snapshot.settings.ai_feature(qnc_ai_features::TRANSCRIPTION).unwrap_or(false) {
            return Ok(None);
        }
        let reader = active.settings_reader().clone();
        let settings = snapshot.settings;
        let db = qnc_db_broker::ProjectDbTarget::for_project(&reader, &settings)?;
        let content = qnc_content_read::ContentReader::for_project(&reader, &settings)?;
        let sources = qnc_source_bindings::load(root)?.sources;
        let target = qnc_content_store::ContentTarget::for_project(&reader, &settings).map_err(|e| e.to_string());
        let clips = Arc::new(qnc_player_clip_content::PlayerClipContent { target });
        let locator = qnc_still_locator::StillLocator::new(reader, settings, sources, clips);
        Ok(Some(Self { db, locator, content }))
    }

    /// Every imported clip not in the queue yet goes in, whole, with the fast engine.
    pub(crate) fn queue_imported(&self) -> Result<usize, String> {
        // A clip that failed because its record was not final yet goes in again (live 2026-11-12:
        // queued once copied, its record still being completed: "player input: NotFinal").
        let queued: Vec<String> = qnc_transcript_db::read_queue(&self.db)?
            .into_iter()
            .filter(|entry| !(entry.state == QueueState::Failed && entry.error.contains("NotFinal")))
            .map(|entry| entry.clip_id)
            .collect();
        let transcribed = qnc_transcript_db::read_transcribed(&self.db)?;
        // Only a clip whose record is final can be heard (its sound is read through the player input).
        let waiting = self.content.records_waiting()?;
        let clips: Vec<_> = self
            .content
            .summaries()?
            .into_iter()
            .filter(|clip| clip.imported && clip.duration_frames > 0 && !waiting.contains(&clip.clip_id))
            .filter(|clip| !queued.contains(&clip.clip_id) && !transcribed.contains(&clip.clip_id))
            .collect();
        let count = clips.len();
        // A long clip goes in as pieces of about a minute, heard by every program at once (user
        // 2026-11-17: the longest clips came last and each was heard alone, 95 s with the processor
        // half idle). The engine hears a second either side of a piece and keeps the words whose
        // middle is inside, so no word is cut at a piece's edge.
        let new: Vec<Queued> = clips
            .into_iter()
            .flat_map(|clip| {
                // Without its length in seconds a clip goes in whole.
                let most = if clip.duration_seconds > 0.0 {
                    ((PIECE_SECONDS * clip.duration_frames as f64 / clip.duration_seconds).round() as u64).max(1)
                } else {
                    clip.duration_frames
                };
                let pieces = clip.duration_frames.div_ceil(most).max(1);
                (0..pieces).map(move |n| Queued {
                    clip_id: clip.clip_id.clone(),
                    from_frame: clip.duration_frames * n / pieces,
                    to_frame: clip.duration_frames * (n + 1) / pieces,
                    quality: QueueQuality::Fast,
                    state: QueueState::Waiting,
                    error: String::new(),
                    took_ms: 0,
                })
            })
            .collect();
        if count > 0 {
            qnc_transcript_db::enqueue(&self.db, new)?;
        }
        Ok(count)
    }

    /// Whether the queue still has clips to hear.
    pub(crate) fn busy(&self) -> bool {
        qnc_transcript_db::read_queue(&self.db)
            .is_ok_and(|queue| queue.iter().any(|entry| matches!(entry.state, QueueState::Waiting | QueueState::Running)))
    }
}

/// Queues the imported clips and keeps the queue's runner going while `running`; afterwards it
/// waits until the queue is heard out (or `stop`). `idle`: nothing waits to be heard now. The
/// clips are queued only once `waves_done` (user 2026-11-19: "val i prijepis zajedno"): those
/// whose wave this run read were heard from its sound by [`Statements`], the queue takes the
/// clips it did not decide.
pub(crate) fn run(root: &Path, running: &std::sync::atomic::AtomicBool, stop: &std::sync::atomic::AtomicBool, idle: &std::sync::atomic::AtomicBool, waves_done: &dyn Fn() -> bool) {
    let transcription = match Transcription::of_active(root) {
        Ok(Some(transcription)) => transcription,
        Ok(None) => return,
        Err(error) => return crate::trace(format!("transcription not started: {error}")),
    };
    let mut runner = qnc_transcribe_queue::QueueRunner::new();
    loop {
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            runner.stop();
            return;
        }
        let more = running.load(std::sync::atomic::Ordering::Relaxed);
        match if waves_done() { transcription.queue_imported() } else { Ok(0) } {
            Ok(0) => {}
            Ok(queued) => crate::trace(format!("transcription queued {queued} clips")),
            Err(error) => crate::trace(format!("transcription queue: {error}")),
        }
        let busy = transcription.busy();
        idle.store(!busy, std::sync::atomic::Ordering::Relaxed);
        if busy {
            runner.ensure(&transcription.locator, &transcription.db);
        } else if !more {
            return;
        }
        let _ = runner.take();
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Which clips the AI steps take: `shots` for the shot analysis, `content` for what the shots show.
pub(crate) struct AiTakes {
    pub(crate) shots: Box<dyn Fn(&str) -> bool + Send + Sync>,
    pub(crate) content: Box<dyn Fn(&str) -> bool + Send + Sync>,
}

/// The shot analysis takes every clip. With transcription on, what the shots show is looked at for a clip that is no statement, and for a statement only when
/// its analysis found more than one shot (user 2026-11-12: "ako se kadar potpuno promijeni to
/// treba analizirati jer se iz toga mogu izvući pokrivalice"); its spoken shot stays whole.
pub(crate) fn ai_takes(root: &Path, stopped: bool) -> Result<AiTakes, String> {
    let Some(transcription) = Transcription::of_active(root)? else { return Ok(AiTakes { shots: Box::new(|_| true), content: Box::new(|_| true) }) };
    let statement = |clip_id: &str, to: u64| -> Result<bool, String> {
        let transcripts = qnc_transcript_db::read_clip(&transcription.db, clip_id)?;
        Ok(qnc_transcript_db::spoken_share(&transcripts, 0, to).is_some_and(|share| share >= qnc_transcript_db::SPOKEN))
    };
    let frames: HashMap<String, u64> = transcription.content.summaries()?.into_iter().map(|clip| (clip.clip_id, clip.duration_frames)).collect();
    // Per clip: (heard, a statement).
    // A long clip is queued in pieces: it is heard when every piece is; a statement is judged on
    // its whole transcript once.
    let mut pieces: HashMap<String, (bool, bool)> = HashMap::new();
    for entry in qnc_transcript_db::read_queue(&transcription.db)? {
        let heard = match entry.state {
            // A clip that failed only because its record was not final is heard again.
            QueueState::Failed => stopped || !entry.error.contains("NotFinal"),
            QueueState::Done => true,
            // The user stopped the transcription: what is not heard is not known as a statement.
            QueueState::Waiting | QueueState::Running => stopped,
        };
        let clip = pieces.entry(entry.clip_id).or_insert((true, false));
        clip.0 &= heard;
        clip.1 |= entry.state == QueueState::Done;
    }
    let mut known: HashMap<String, (bool, bool)> = HashMap::new();
    for (clip, (heard, any_done)) in pieces {
        let spoken = heard && any_done && statement(&clip, frames.get(&clip).copied().unwrap_or(0))?;
        known.insert(clip, (heard, spoken));
    }
    // A clip transcribed before (by hand) without a queue entry is judged by its transcript.
    for clip in qnc_transcript_db::read_transcribed(&transcription.db)? {
        if !known.contains_key(&clip) {
            let to = frames.get(&clip).copied().unwrap_or(0);
            known.insert(clip.clone(), (true, statement(&clip, to)?));
        }
    }
    let db = transcription.db.clone();
    Ok(AiTakes {
        // The shots do not wait for the transcription (user 2026-11-14: the AI shots ended 7.5
        // min after it): every clip is analysed once copied; a statement's analysis is only used
        // to find a scene change in it.
        shots: Box::new(|_| true),
        content: Box::new(move |clip| match known.get(clip) {
            None => stopped,
            Some((false, _)) => false,
            Some((true, false)) => true,
            // A statement: only when its analysis found the scene change.
            Some((true, true)) => qnc_shot_db::read_clip(&db, clip).ok().flatten().is_some_and(|shots| shots.shots.len() >= 2),
        }),
    })
}

/// The statements heard from the sound the waves read (user 2026-11-19: "val i prijepis
/// zajedno"; "prijepisi samo izjava s prvim kanalom"): each clip's first channel, handed over by
/// the wave worker once its wave is made, waits here at 16 kHz; as many hearers as the fast
/// engine has programs take them, the clip a preview has open first. A statement's transcript is
/// kept and its queue entry done; a clip that is no statement is marked so in the queue (the AI
/// shots take it, the queue does not hear it again). Nothing is read from the medium again.
pub(crate) struct Statements {
    waiting: Arc<Waiting>,
    hearers: Vec<std::thread::JoinHandle<()>>,
}

#[derive(Default)]
struct Waiting {
    /// The clips to hear and whether no more come.
    clips: std::sync::Mutex<(Vec<(String, Vec<f32>)>, bool)>,
    ready: std::sync::Condvar,
    /// How many clips are being heard now.
    hearing: std::sync::atomic::AtomicUsize,
}

impl Statements {
    /// The hearers of the active project; none when its AI has no transcription. `stopped`: the
    /// user stopped the transcription (what waits is dropped).
    pub(crate) fn start(root: &Path, stopped: Arc<dyn Fn() -> bool + Send + Sync>) -> Result<Option<Self>, String> {
        let Some(transcription) = Transcription::of_active(root)? else { return Ok(None) };
        let transcription = Arc::new(transcription);
        let preview = qnc_content_store::ContentTarget::for_project(transcription.locator.reader(), transcription.locator.settings()).ok();
        let waiting = Arc::new(Waiting::default());
        let count = qnc_ai_catalog::installed_fast_transcribers().map_or(1, |engines| engines.iter().map(|engine| engine.parallel.max(1) as usize).sum::<usize>()).max(1);
        let hearers = (0..count)
            .map(|_| {
                let (transcription, waiting, preview, stopped) = (transcription.clone(), waiting.clone(), preview.clone(), stopped.clone());
                std::thread::spawn(move || loop {
                    let Some((clip_id, samples)) = waiting.next(preview.as_ref()) else { return };
                    if !stopped() {
                        transcription.hear(&clip_id, &samples);
                    }
                    waiting.hearing.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                })
            })
            .collect();
        Ok(Some(Self { waiting, hearers }))
    }

    /// What the wave worker hands a clip's first channel to.
    pub(crate) fn sink(&self) -> qnc_wave_worker::FirstChannelSink {
        let waiting = self.waiting.clone();
        Arc::new(move |first: qnc_wave_worker::FirstChannel| match qnc_transcribe_jobs::to_whisper_rate(&first.samples, first.rate) {
            Ok(samples) => {
                let mut clips = waiting.clips.lock().unwrap_or_else(|e| e.into_inner());
                clips.0.push((first.clip_id, samples));
                waiting.ready.notify_one();
            }
            Err(error) => crate::trace(format!("statement {} not heard: {error}", first.clip_id)),
        })
    }

    /// Whether a clip waits or is being heard.
    pub(crate) fn busy(&self) -> bool {
        !self.waiting.clips.lock().unwrap_or_else(|e| e.into_inner()).0.is_empty() || self.waiting.hearing.load(std::sync::atomic::Ordering::Relaxed) > 0
    }
}

impl Drop for Statements {
    /// No more clips come: the hearers hear what waits (nothing once stopped) and end.
    fn drop(&mut self) {
        {
            let mut clips = self.waiting.clips.lock().unwrap_or_else(|e| e.into_inner());
            clips.1 = true;
            self.waiting.ready.notify_all();
        }
        for hearer in self.hearers.drain(..) {
            let _ = hearer.join();
        }
    }
}

impl Waiting {
    /// The next clip to hear, the one a preview has open first; none once no more come.
    fn next(&self, preview: Option<&qnc_content_store::ContentTarget>) -> Option<(String, Vec<f32>)> {
        let mut clips = self.clips.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            if !clips.0.is_empty() {
                let shown = preview.and_then(|target| qnc_playback_activity::read(target, qnc_playback_activity::PLAYBACK_CLIP));
                let at = shown.and_then(|shown| clips.0.iter().position(|(clip, _)| *clip == shown)).unwrap_or(0);
                self.hearing.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Some(clips.0.remove(at));
            }
            if clips.1 {
                return None;
            }
            clips = self.ready.wait(clips).unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Transcription {
    /// Hears `clip_id`'s first channel (16 kHz): a statement is kept, a clip that is no
    /// statement is marked so; a failure leaves the clip to the queue.
    fn hear(&self, clip_id: &str, samples: &[f32]) {
        let began = std::time::Instant::now();
        let never = std::sync::atomic::AtomicBool::new(false);
        let heard = qnc_transcribe_run::hear_statement(&self.locator, clip_id, samples, &never);
        let took = began.elapsed().as_millis() as u64;
        let (state, error, end) = match heard {
            Ok((Some(transcript), end)) => {
                if let Err(error) = qnc_transcript_db::save(&self.db, transcript) {
                    return crate::trace(format!("statement {clip_id} not kept: {error}"));
                }
                (QueueState::Done, String::new(), end)
            }
            Ok((None, end)) => (QueueState::Failed, qnc_ui_text::text("transcribe.no_statement").to_string(), end),
            Err(error) => return crate::trace(format!("statement {clip_id} not heard: {error}")),
        };
        crate::trace(format!("statement {clip_id} {} after {took} ms ({:.0} s of sound)", if state == QueueState::Done { "transcribed" } else { "is no statement" }, samples.len() as f64 / f64::from(qnc_transcribe_jobs::WHISPER_RATE)));
        let entry = Queued { clip_id: clip_id.to_string(), from_frame: 0, to_frame: end.max(1), quality: QueueQuality::Fast, state: QueueState::Waiting, error: String::new(), took_ms: 0 };
        let marked = qnc_transcript_db::enqueue(&self.db, vec![entry.clone()]).and_then(|_| qnc_transcript_db::set_queued(&self.db, &entry, state, &error, took));
        if let Err(error) = marked {
            crate::trace(format!("statement {clip_id} not marked: {error}"));
        }
    }
}
