//! Streaming transcription (user 2026-11-16: "striming je bitan, jer novinar neće zbog stiske s
//! vremenom moći čekati AI agente; striming agent pomaže novinaru jer ne mora prepisivati ručno
//! izjavu"). While the source timeline plays a clip, its sound is transcribed in pieces of a few
//! seconds from where the play started, a little ahead of the playhead, with the best engine of
//! the project's language (the Croatian model for Croatian), and each piece is kept in the
//! project's database at once: the transcript panel reads it from there (the one truth), so the
//! text comes as the clip plays. A part a Croatian transcript already says is not heard again.
//! It stops with the play, or when another clip is shown. It draws nothing.

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

/// The streaming of one play.
struct Running {
    clip: String,
    /// The playhead, for how far ahead to go.
    playhead: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    /// The play stopped: the pieces up to the playhead are finished, then it ends.
    played: Arc<AtomicBool>,
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
    /// Follows the source preview: `clip` shown, the confirmed `frame` and whether it `plays`.
    /// A play starts a streaming from its frame; the playhead leads it; a stop lets it finish up
    /// to where the play stopped; another clip ends it.
    pub fn follow(&mut self, locator: Option<&StillLocator>, clip: Option<&str>, frame: Option<u64>, plays: bool) {
        if let Some(running) = &self.running {
            if Some(running.clip.as_str()) != clip {
                self.running = None;
            }
        }
        match (&self.running, frame) {
            (Some(running), Some(frame)) => {
                running.playhead.store(frame, Ordering::Relaxed);
                if !plays {
                    running.played.store(true, Ordering::Relaxed);
                }
                if plays && running.played.load(Ordering::Relaxed) {
                    // Played again: a new streaming from here.
                    self.running = None;
                }
            }
            _ => {}
        }
        if self.running.is_some() || !plays {
            return;
        }
        let (Some(locator), Some(clip), Some(frame)) = (locator, clip, frame) else { return };
        if !locator.settings().ai_feature(qnc_ai_features::TRANSCRIPTION).unwrap_or(false) {
            return;
        }
        let running = Running { clip: clip.to_string(), playhead: Arc::new(AtomicU64::new(frame)), stop: Arc::new(AtomicBool::new(false)), played: Arc::new(AtomicBool::new(false)) };
        let (locator, clip_id, playhead, stop, played) = (locator.clone(), clip.to_string(), running.playhead.clone(), running.stop.clone(), running.played.clone());
        let spawned = std::thread::Builder::new().name("qnc-stream-transcript".into()).spawn(move || {
            if let Err(error) = stream(&locator, &clip_id, frame, &playhead, &stop, &played) {
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

/// The pieces of `clip_id` from `start` on, each kept as it is heard.
fn stream(locator: &StillLocator, clip_id: &str, start: u64, playhead: &AtomicU64, stop: &AtomicBool, played: &AtomicBool) -> Result<(), String> {
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
    let heard: Vec<(u64, u64)> = qnc_transcript_db::read_spans(&target)?
        .into_iter()
        .filter(|span| span.clip_id == clip_id && fine.as_deref().is_some_and(|fine| span.engine.contains(fine)))
        .map(|span| (span.from_frame, span.to_frame))
        .collect();
    let mut at = start;
    loop {
        if stop.load(Ordering::Relaxed) || end.is_some_and(|end| at >= end) {
            return Ok(());
        }
        let head = playhead.load(Ordering::Relaxed);
        if played.load(Ordering::Relaxed) && at >= head {
            return Ok(());
        }
        if at > head + AHEAD_SECONDS * second {
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }
        let to = end.map_or(at + PIECE_SECONDS * second, |end| (at + PIECE_SECONDS * second).min(end));
        if let Some(&(_, heard_to)) = heard.iter().find(|(from, heard_to)| *from <= at && at < *heard_to) {
            at = heard_to;
            continue;
        }
        let ask = qnc_transcribe_run::Ask { clip_id: clip_id.to_string(), from: at.saturating_sub(LEAD_SECONDS * second), to };
        let mut transcript = qnc_transcribe_run::transcribe(locator, &ask, quality, qnc_transcribe_run::Pace::Now, stop, &mut |_| {}, &mut |_| {})?;
        transcript.lines = qnc_transcript_db::lines_cut(&transcript.lines, at, to, true);
        (transcript.from_frame, transcript.to_frame) = (at, to);
        qnc_transcript_db::save(&target, transcript)?;
        at = to;
    }
}
