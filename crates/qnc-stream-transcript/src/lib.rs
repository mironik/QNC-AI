//! Streaming transcription (user 2026-11-16: "striming je bitan, jer novinar neće zbog stiske s
//! vremenom moći čekati AI agente; striming agent pomaže novinaru jer ne mora prepisivati ručno
//! izjavu"). From the moment a clip is shown on the source timeline, playing or paused, its sound
//! is transcribed in pieces of a few seconds from the playhead on, up to a little ahead of it, and
//! each piece is kept in the project's database at once: the transcript panel reads it from there
//! (the one truth), so the text comes as the clip is heard.
//!
//! The live engine of the AI catalog hears the pieces (user 2026-11-19: Parakeet; Nemotron did not
//! keep up beside the player, and the Croatian model in a pause wrote its lower case over
//! Parakeet's sentences); a catalog without one hears with the fine engine of the language. A
//! playhead that jumps takes the pieces with it. A part any transcript already says (the
//! statements heard with the waves) is not heard again. Another clip ends it. It draws nothing.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use qnc_still_locator::StillLocator;
use qnc_transcribe_run::Quality;

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
    /// Whether the clip plays (the live engine) or not (the fine one).
    plays: Arc<AtomicBool>,
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
    /// Follows the source preview: `clip` shown, its confirmed `frame` and whether it `plays`. The
    /// clip is heard from the moment it is shown, its pieces ahead of the playhead: by the live
    /// engine while it plays, by the fine one in a pause; another clip ends it.
    pub fn follow(&mut self, locator: Option<&StillLocator>, clip: Option<&str>, frame: Option<u64>, plays: bool) {
        if let Some(running) = &self.running {
            if Some(running.clip.as_str()) != clip {
                self.running = None;
            }
        }
        // A project open: the engines are loaded before a clip is played.
        if let Some(locator) = locator {
            warm(locator);
        }
        if let (Some(running), Some(frame)) = (&self.running, frame) {
            running.playhead.store(frame, Ordering::Relaxed);
            running.plays.store(plays, Ordering::Relaxed);
        }
        if self.running.is_some() {
            return;
        }
        let (Some(locator), Some(clip), Some(frame)) = (locator, clip, frame) else { return };
        if !locator.settings().ai_feature(qnc_ai_features::TRANSCRIPTION).unwrap_or(false) {
            return;
        }
        let running = Running {
            clip: clip.to_string(),
            playhead: Arc::new(AtomicU64::new(frame)),
            plays: Arc::new(AtomicBool::new(plays)),
            stop: Arc::new(AtomicBool::new(false)),
        };
        let (locator, clip_id, playhead, plays, stop) = (locator.clone(), clip.to_string(), running.playhead.clone(), running.plays.clone(), running.stop.clone());
        let spawned = std::thread::Builder::new().name("qnc-stream-transcript".into()).spawn(move || {
            if let Err(error) = stream(&locator, &clip_id, &playhead, &plays, &stop) {
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

/// The fine engine of `language`: the Croatian model for Croatian, the fast one otherwise.
fn fine_quality(language: &str) -> Quality {
    if language == "hr" {
        Quality::Best
    } else {
        Quality::Fast
    }
}

/// The pieces of `clip_id` ahead of the playhead, each kept as it is heard: from where the
/// playhead is, up to `AHEAD_SECONDS` ahead of it; when the playhead jumps out of what is being
/// heard, the pieces go on from it. While the clip plays the live engine hears them (what any
/// transcript of this streaming already says is skipped), in a pause the fine one (only what a
/// fine transcript says is skipped).
fn stream(locator: &StillLocator, clip_id: &str, playhead: &AtomicU64, plays: &AtomicBool, stop: &AtomicBool) -> Result<(), String> {
    let settings = locator.settings();
    let language = settings.speech_language().map_err(|error| error.to_string())?;
    let fine_quality = fine_quality(&language);
    let live = qnc_ai_catalog::installed_transcriber_for(Quality::Live).is_ok();
    let place = locator.locate_sound(clip_id)?;
    let second = u64::from(place.fps.0 / place.fps.1.max(1)).max(1);
    let end = place.media.streams.iter().find_map(|stream| match &stream.details {
        qnc_media_metadata::StreamDetails::Video(video) => video.exact_frame_count(),
        _ => None,
    });
    let target = qnc_db_broker::ProjectDbTarget::for_project(locator.reader(), settings)?;
    // What any transcript already says is not heard again (Parakeet's of the import, the statements).
    let mut fine_heard: Vec<(u64, u64)> = qnc_transcript_db::read_spans(&target)?
        .into_iter()
        .filter(|span| span.clip_id == clip_id)
        .map(|span| (span.from_frame, span.to_frame))
        .collect();
    // What the live engine heard in this streaming (a pause hears it again, finely).
    let mut live_heard: Vec<(u64, u64)> = Vec::new();
    let mut at = playhead.load(Ordering::Relaxed);
    let mut was_live = live && plays.load(Ordering::Relaxed);
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let head = playhead.load(Ordering::Relaxed);
        // The live engine in a pause too (user 2026-11-19: the Croatian model in a pause wrote its
        // lower case over Parakeet's sentences); without one, the fine engine.
        let now_live = live;
        let _ = plays;
        // The engine changed (play or pause): from the playhead on.
        if now_live != was_live {
            at = head;
            was_live = now_live;
        }
        // The playhead jumped back before what is heard, or far past it: from the playhead on.
        if head + second < at.saturating_sub(AHEAD_SECONDS * second) || head > at + second {
            at = head;
        }
        if at > head + AHEAD_SECONDS * second || end.is_some_and(|end| at >= end) {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        }
        let skip = |spans: &[(u64, u64)]| spans.iter().find(|(from, heard_to)| *from <= at && at < *heard_to).map(|&(_, to)| to);
        if let Some(heard_to) = skip(&fine_heard).or_else(|| if now_live { skip(&live_heard) } else { None }) {
            at = heard_to;
            continue;
        }
        let quality = if now_live { Quality::Live } else { fine_quality };
        let to = end.map_or(at + PIECE_SECONDS * second, |end| (at + PIECE_SECONDS * second).min(end));
        let ask = qnc_transcribe_run::Ask { clip_id: clip_id.to_string(), from: at.saturating_sub(LEAD_SECONDS * second), to };
        let began = std::time::Instant::now();
        // A piece ends when the streaming ends or the clip starts or stops playing (live
        // 2026-11-19: a fine piece begun in a pause took 23 s once the player played): the next
        // piece is then heard by the engine of the new state.
        let (cut, done) = (AtomicBool::new(false), AtomicBool::new(false));
        let heard = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !done.load(Ordering::Relaxed) {
                    if stop.load(Ordering::Relaxed) {
                        cut.store(true, Ordering::Relaxed);
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            });
            let heard = qnc_transcribe_run::transcribe(locator, &ask, quality, qnc_transcribe_run::Pace::Now, &cut, &mut |_| {}, &mut |_| {});
            done.store(true, Ordering::Relaxed);
            heard
        });
        let mut transcript = match heard {
            Ok(transcript) => transcript,
            Err(_) if cut.load(Ordering::Relaxed) => {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("stream-transcript {clip_id} {quality:?} {at}..{to} cut after ms={}", began.elapsed().as_millis()));
                continue;
            }
            Err(error) => return Err(error),
        };
        transcript.lines = qnc_transcript_db::lines_cut(&transcript.lines, at, to, true);
        (transcript.from_frame, transcript.to_frame) = (at, to);
        qnc_dev_diagnostics::log_line(
            qnc_dev_diagnostics::DiagnosticsStream::Player,
            format!("stream-transcript {clip_id} {quality:?} {at}..{to} ms={} lines={}", began.elapsed().as_millis(), transcript.lines.len()),
        );
        qnc_transcript_db::save(&target, transcript)?;
        if now_live {
            live_heard.push((at, to));
        } else {
            fine_heard.push((at, to));
        }
        at = to;
    }
}

/// Loads the engines' models once per process, before the first play (user 2026-11-18: the first
/// piece waited 19 s for it): one second of silence heard by each on a thread of its own.
fn warm(locator: &StillLocator) {
    static WARMED: AtomicBool = AtomicBool::new(false);
    let settings = locator.settings();
    if !settings.ai_feature(qnc_ai_features::TRANSCRIPTION).unwrap_or(false) || WARMED.swap(true, Ordering::Relaxed) {
        return;
    }
    let Ok(language) = settings.speech_language() else { return };
    // The transcription programs this form starts stay at normal priority (the live transcript
    // keeps up with the player; the background worker's own stay below normal).
    std::env::set_var("QNC_TRANSCRIBE_PRIORITY", "normal");
    // The engine the streaming hears with: the live one, else the fine one.
    let qualities = if qnc_ai_catalog::installed_transcriber_for(Quality::Live).is_ok() { vec![Quality::Live] } else { vec![fine_quality(&language)] };
    // One after the other, the live engine first (live 2026-11-19: loaded at the same time each
    // took 25-40 s).
    let _ = std::thread::Builder::new().name("qnc-stream-transcript-warm".into()).spawn(move || {
        for quality in qualities {
            let silence = vec![0.0f32; qnc_transcribe_jobs::WHISPER_RATE as usize];
            let began = std::time::Instant::now();
            let result = qnc_transcribe_run::hear_samples_with(quality, &silence, &language, &AtomicBool::new(false));
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("stream-transcript warm {quality:?} ms={} ok={}", began.elapsed().as_millis(), result.is_ok()));
        }
    });
}
