//! Streaming transcription (user 2026-11-16: "striming je bitan, jer novinar neće zbog stiske s
//! vremenom moći čekati AI agente; striming agent pomaže novinaru jer ne mora prepisivati ručno
//! izjavu"). From the moment a clip is shown on the source timeline, playing or paused, its sound
//! is transcribed in pieces of a few seconds from the playhead on, up to a little ahead of it, with
//! the best engine of the project's language (the Croatian model for Croatian), and each piece is
//! kept in the project's database at once: the transcript panel reads it from there (the one
//! truth), so the text comes as the clip is heard. A pause leaves the processor to it (user
//! 2026-11-18: on the laptop it did not keep up with a play of the original); a playhead that jumps
//! takes the pieces with it. A part a fine transcript already says is not heard again. Another clip
//! ends it. It draws nothing.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use qnc_still_locator::StillLocator;

/// How many seconds of sound one piece is.
const PIECE_SECONDS: u64 = 5;
/// The sound heard before a piece, so a word at its start is heard whole (only words whose
/// middle is in the piece are kept).
const LEAD_SECONDS: u64 = 1;
/// How far ahead of the playhead the pieces go.
const AHEAD_SECONDS: u64 = 12;

/// The streaming of one clip.
struct Running {
    clip: String,
    /// The playhead, for where the pieces go.
    playhead: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub struct StreamTranscript {
    running: Option<Running>,
}

impl StreamTranscript {
    /// Follows the source preview: `clip` shown and its confirmed `frame`. The clip is heard from
    /// the moment it is shown, playing or paused (user 2026-11-18: wav2vec2 did not keep up with a
    /// play of the original on the laptop; a pause leaves the processor to it), its pieces ahead of
    /// the playhead; another clip ends it. `plays` is kept for the caller's sake.
    pub fn follow(&mut self, locator: Option<&StillLocator>, clip: Option<&str>, frame: Option<u64>, _plays: bool) {
        if let Some(running) = &self.running {
            if Some(running.clip.as_str()) != clip {
                self.running = None;
            }
        }
        // A clip shown: the engine is loaded before it is needed.
        if let (Some(locator), Some(_)) = (locator, clip) {
            warm(locator);
        }
        if let (Some(running), Some(frame)) = (&self.running, frame) {
            running.playhead.store(frame, Ordering::Relaxed);
        }
        if self.running.is_some() {
            return;
        }
        let (Some(locator), Some(clip), Some(frame)) = (locator, clip, frame) else { return };
        if !locator.settings().ai_feature(qnc_ai_features::TRANSCRIPTION).unwrap_or(false) {
            return;
        }
        let running = Running { clip: clip.to_string(), playhead: Arc::new(AtomicU64::new(frame)), stop: Arc::new(AtomicBool::new(false)) };
        let (locator, clip_id, playhead, stop) = (locator.clone(), clip.to_string(), running.playhead.clone(), running.stop.clone());
        let spawned = std::thread::Builder::new().name("qnc-stream-transcript".into()).spawn(move || {
            if let Err(error) = stream(&locator, &clip_id, &playhead, &stop) {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("stream-transcript {clip_id} stopped: {error}"));
            }
        });
        if spawned.is_ok() {
            self.running = Some(running);
        }
    }

    /// Whether a streaming runs.
    pub fn running(&self) -> bool {
        self.running.is_some()
    }
}

/// The pieces of `clip_id` ahead of the playhead, each kept as it is heard: from where the
/// playhead is, up to `AHEAD_SECONDS` ahead of it; when the playhead jumps out of what is being
/// heard, the pieces go on from it. What a fine transcript already says is skipped.
fn stream(locator: &StillLocator, clip_id: &str, playhead: &AtomicU64, stop: &AtomicBool) -> Result<(), String> {
    let settings = locator.settings();
    let language = settings.speech_language().map_err(|error| error.to_string())?;
    let quality = if language == "hr" { qnc_transcribe_run::Quality::Best } else { qnc_transcribe_run::Quality::Fast };
    let place = locator.locate_sound(clip_id)?;
    let second = u64::from(place.fps.0 / place.fps.1.max(1)).max(1);
    let end = place.media.streams.iter().find_map(|stream| match &stream.details {
        qnc_media_metadata::StreamDetails::Video(video) => video.exact_frame_count(),
        _ => None,
    });
    let target = qnc_db_broker::ProjectDbTarget::for_project(locator.reader(), settings)?;
    // What a transcript of the fine engine already says is not heard again.
    let fine = qnc_ai_catalog::installed_transcriber_for(quality).ok().and_then(|engine| engine.model.file_name().map(|name| name.to_string_lossy().into_owned()));
    let mut heard: Vec<(u64, u64)> = qnc_transcript_db::read_spans(&target)?
        .into_iter()
        .filter(|span| span.clip_id == clip_id && fine.as_deref().is_some_and(|fine| span.engine.contains(fine)))
        .map(|span| (span.from_frame, span.to_frame))
        .collect();
    let mut at = playhead.load(Ordering::Relaxed);
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let head = playhead.load(Ordering::Relaxed);
        // The playhead jumped back before what is heard, or far past it: from the playhead on.
        if head + second < at.saturating_sub(AHEAD_SECONDS * second) || head > at + second {
            at = head;
        }
        if at > head + AHEAD_SECONDS * second || end.is_some_and(|end| at >= end) {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }
        if let Some(&(_, heard_to)) = heard.iter().find(|(from, heard_to)| *from <= at && at < *heard_to) {
            at = heard_to;
            continue;
        }
        let to = end.map_or(at + PIECE_SECONDS * second, |end| (at + PIECE_SECONDS * second).min(end));
        let ask = qnc_transcribe_run::Ask { clip_id: clip_id.to_string(), from: at.saturating_sub(LEAD_SECONDS * second), to };
        let mut transcript = qnc_transcribe_run::transcribe(locator, &ask, quality, qnc_transcribe_run::Pace::Now, stop, &mut |_| {}, &mut |_| {})?;
        transcript.lines = qnc_transcript_db::lines_cut(&transcript.lines, at, to, true);
        (transcript.from_frame, transcript.to_frame) = (at, to);
        qnc_transcript_db::save(&target, transcript)?;
        heard.push((at, to));
        at = to;
    }
}

/// Loads the fine engine's model once per process, before the first play (user 2026-11-18: the
/// first piece waited 19 s for it): one second of silence heard on a thread of its own.
fn warm(locator: &StillLocator) {
    static WARMED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let settings = locator.settings();
    if !settings.ai_feature(qnc_ai_features::TRANSCRIPTION).unwrap_or(false) || WARMED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Ok(language) = settings.speech_language() else { return };
    if language != "hr" {
        return;
    }
    let _ = std::thread::Builder::new().name("qnc-stream-transcript-warm".into()).spawn(move || {
        let silence = vec![0.0f32; qnc_transcribe_jobs::WHISPER_RATE as usize];
        let began = std::time::Instant::now();
        let result = qnc_transcribe_run::hear_samples(&silence, &language, &AtomicBool::new(false));
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("stream-transcript warm ms={} ok={}", began.elapsed().as_millis(), result.is_ok()));
    });
}
