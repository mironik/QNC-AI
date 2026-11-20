//! The OUT marked by words of the transcript is the end of the last word's voice, not its
//! transcribed end (user 2026-11-20: "OUT na kraj glasa riječi"; "stanovnika" given to 1006,
//! its voice on to 1010, the OUT cut the word). The voice is read from the original (the fine
//! wave, in the background) and the OUT moved on to where it falls; the IN and anything the
//! user set afterwards are never touched.

use qnc_source_preview::SourcePreview;
use qnc_timeline_model::TimelineProjection;

/// Frames read before the transcribed end and after it (the voice may run on that far).
const BEFORE: u64 = 20;
const AFTER: u64 = 30;

#[derive(Default)]
pub(crate) struct VoiceEnd {
    /// The clip, the word's transcribed end and the OUT it set, until the voice is read.
    asked: Option<(String, u64, u64)>,
    fine: qnc_fine_wave::FineWave,
}

impl std::fmt::Debug for VoiceEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VoiceEnd").field("asked", &self.asked).finish()
    }
}

impl VoiceEnd {
    /// An OUT was set at the transcribed end `end` of a word of `clip`.
    pub(crate) fn ask(&mut self, clip: Option<&str>, end: u64) {
        self.asked = clip.map(|clip| (clip.to_string(), end, end));
    }

    /// Once a frame: when the voice around the word's end is read, the OUT goes on to its end.
    pub(crate) fn settle(&mut self, clip: Option<&str>, channel: u16, preview: &SourcePreview, timeline: &mut TimelineProjection) {
        let Some((asked_clip, end, out)) = self.asked.clone() else { return };
        // Another clip, or the user moved the OUT since: the asking is over.
        if clip != Some(asked_clip.as_str()) || timeline.source_out_frame != Some(out) {
            self.asked = None;
            return;
        }
        let Some(locator) = preview.still_locator() else { return };
        let length = timeline.range_start() + timeline.duration_frames();
        let (from, to) = (end.saturating_sub(BEFORE), (end + AFTER).min(length.max(end + 1)));
        let placement = qnc_program_waveform::Placement {
            clip_id: asked_clip.clone(),
            channel,
            program_start: 0,
            program_end: length.max(to),
            source_in: 0,
            source_out: length.max(to),
            level_db: 0,
            off: false,
            curve: Vec::new(),
        };
        let (peaks, complete) = self.fine.lane_peaks(&locator, &[placement], from, to, (to - from) as usize);
        if !complete {
            return;
        }
        self.asked = None;
        let voice = voice_end(from, &peaks, end);
        if voice > out {
            if let Some(in_frame) = timeline.source_in_frame {
                *timeline = timeline.clone().with_second_cut(in_frame, voice);
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("source-voice-end clip={asked_clip} out {out} -> {voice}"));
            }
        }
    }
}

/// The farthest the OUT goes past the transcribed end (frames): continuous speech goes on into
/// the next word, so the voice is followed no further than its own tail.
const TAIL: u64 = 10;
/// Quiet this long ends the voice (frames).
const QUIET: u64 = 3;

/// Where the voice of a word transcribed to end at `end` falls (`peaks`: one per frame from
/// `from`, 0..1): on past `end` while it is voice, through a dip shorter than `QUIET`, at most
/// `TAIL` frames. Voice is 6 dB over the quietest fifth and no more than 18 dB under the loud.
fn voice_end(from: u64, peaks: &[f32], end: u64) -> u64 {
    if peaks.is_empty() || end < from {
        return end;
    }
    let mut sorted = peaks.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let (quiet, loud) = (sorted[sorted.len() / 5], sorted[sorted.len() * 9 / 10]);
    let threshold = (quiet * 2.0).min(loud * 0.5).max(loud / 8.0).max(0.008);
    let loud = |frame: u64| peaks.get((frame - from) as usize).is_some_and(|peak| *peak > threshold);
    let mut voice = end;
    let mut quiet = 0;
    for frame in end..(end + TAIL).min(from + peaks.len() as u64) {
        if loud(frame) {
            voice = frame + 1;
            quiet = 0;
        } else {
            quiet += 1;
            if quiet >= QUIET {
                break;
            }
        }
    }
    voice
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_out_goes_on_to_where_the_voice_falls_never_into_the_next_word() {
        // "stanovnika" transcribed to 1006, its voice on to 1010, quiet, then the next word at 1030.
        let from = 986;
        let peaks: Vec<f32> = (from..1036).map(|frame| if frame < 1010 || frame >= 1030 { 0.3 } else { 0.002 }).collect();
        assert_eq!(super::voice_end(from, &peaks, 1006), 1010);
        // Continuous speech: no further than its tail.
        let speech = vec![0.3; 50];
        assert_eq!(super::voice_end(from, &speech, 1006), 1016);
    }
}
