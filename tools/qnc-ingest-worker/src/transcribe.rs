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

/// The project of the active project's transcription, read once a run.
pub(crate) struct Transcription {
    db: qnc_db_broker::ProjectDbTarget,
    locator: qnc_still_locator::StillLocator,
    content: qnc_content_read::ContentReader,
}

impl Transcription {
    /// The transcription of the active project; `None` when its AI has no transcription.
    pub(crate) fn of_active(root: &Path) -> Result<Option<Self>, String> {
        let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
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
        let queued: Vec<String> = qnc_transcript_db::read_queue(&self.db)?.into_iter().map(|entry| entry.clip_id).collect();
        let transcribed = qnc_transcript_db::read_transcribed(&self.db)?;
        let new: Vec<Queued> = self
            .content
            .summaries()?
            .into_iter()
            .filter(|clip| clip.imported && clip.duration_frames > 0 && !queued.contains(&clip.clip_id) && !transcribed.contains(&clip.clip_id))
            .map(|clip| Queued {
                clip_id: clip.clip_id,
                from_frame: 0,
                to_frame: clip.duration_frames,
                quality: QueueQuality::Fast,
                state: QueueState::Waiting,
                error: String::new(),
                took_ms: 0,
            })
            .collect();
        let count = new.len();
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
/// waits until the queue is heard out (or `stop`).
pub(crate) fn run(root: &Path, running: &std::sync::atomic::AtomicBool, stop: &std::sync::atomic::AtomicBool) {
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
        match transcription.queue_imported() {
            Ok(0) => {}
            Ok(queued) => crate::trace(format!("transcription queued {queued} clips")),
            Err(error) => crate::trace(format!("transcription queue: {error}")),
        }
        if transcription.busy() {
            runner.ensure(&transcription.locator, &transcription.db);
        } else if !more {
            return;
        }
        let _ = runner.take();
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// Which clips the AI shots may look at: with transcription off every clip; with it on, a clip
/// whose transcription is done and whose words cover less than a statement's share, or whose
/// transcription failed (nothing is known of its speech). A clip still waiting is not taken yet.
pub(crate) fn ai_takes(root: &Path, stopped: bool) -> Result<Box<dyn Fn(&str) -> bool>, String> {
    let Some(transcription) = Transcription::of_active(root)? else { return Ok(Box::new(|_| true)) };
    let statement = |clip_id: &str, to: u64| -> Result<bool, String> {
        let transcripts = qnc_transcript_db::read_clip(&transcription.db, clip_id)?;
        Ok(qnc_transcript_db::spoken_share(&transcripts, 0, to).is_some_and(|share| share >= qnc_transcript_db::SPOKEN))
    };
    let frames: HashMap<String, u64> = transcription.content.summaries()?.into_iter().map(|clip| (clip.clip_id, clip.duration_frames)).collect();
    let mut taken = HashMap::new();
    for entry in qnc_transcript_db::read_queue(&transcription.db)? {
        let take = match entry.state {
            QueueState::Failed => true,
            QueueState::Done => !statement(&entry.clip_id, frames.get(&entry.clip_id).copied().unwrap_or(entry.to_frame))?,
            // The user stopped the transcription: what is not heard is not known as a statement.
            QueueState::Waiting | QueueState::Running => stopped,
        };
        taken.insert(entry.clip_id, take);
    }
    // A clip transcribed before (by hand) without a queue entry is judged by its transcript.
    for clip in qnc_transcript_db::read_transcribed(&transcription.db)? {
        if !taken.contains_key(&clip) {
            let to = frames.get(&clip).copied().unwrap_or(0);
            taken.insert(clip.clone(), !statement(&clip, to)?);
        }
    }
    Ok(Box::new(move |clip| taken.get(clip).copied().unwrap_or(stopped)))
}
