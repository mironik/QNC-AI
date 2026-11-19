//! The letter times of a statement (user 2026-11-19: "Dodaj izjavu ili dodaj složenu izjavu
//! pokreće i prijepis tog dijela, jer ako korisnik krene na fino podešavanje reza u Zumu, treba mu
//! precizno poravnanje teksta i vala"). A statement added is heard again by the Croatian model;
//! once its corrected transcript is kept, the words of its range get their letter times
//! (`qnc-letter-times`), so the trim zoom shows the words where they are said at once, not only
//! when it is opened. It draws nothing and reads nothing itself: the caller hands it the kept
//! transcripts of the clip.

use qnc_still_locator::StillLocator;
use qnc_transcript_db::ClipTranscript;

/// A statement waiting for its letters: its clip, its range and the clip's frames a second.
#[derive(Debug, Clone, PartialEq)]
struct Asked {
    clip_id: String,
    from: u64,
    to: u64,
    fps: f64,
}

#[derive(Default)]
pub struct StatementLetters {
    asked: Vec<Asked>,
    letter_times: qnc_letter_times::LetterTimes,
    /// The statement whose words are being laid now.
    laying: Option<Asked>,
}

impl StatementLetters {
    pub fn new() -> Self {
        Self::default()
    }

    /// `[from, to)` of `clip_id` (at `fps` frames a second) gets its letter times once a fine
    /// transcript of it is kept.
    pub fn ask(&mut self, clip_id: &str, from: u64, to: u64, fps: f64) {
        let asked = Asked { clip_id: clip_id.to_string(), from, to, fps };
        if !self.asked.contains(&asked) {
            self.asked.push(asked);
        }
    }

    /// Lays the letters of the first statement asked whose fine transcript `kept` gives (the kept
    /// transcripts of a clip, `None` while not read), while the player is `idle`. True when a run
    /// was kept (the letters are read again by whoever shows them).
    pub fn poll(&mut self, locator: &StillLocator, kept: &dyn Fn(&str) -> Option<Vec<ClipTranscript>>, idle: bool) -> bool {
        if self.laying.is_some() {
            // Only what is being laid is taken; nothing new is asked (the same words would be
            // laid again).
            let laid = self.letter_times.poll(locator, &[], idle);
            if !self.letter_times.working() {
                self.laying = None;
            }
            return laid;
        }
        let Some(at) = self.asked.iter().position(|asked| kept(&asked.clip_id).is_some_and(|kept| fine_over(&kept, asked))) else { return false };
        let Some(need) = need_of(&self.asked[at], kept) else {
            self.asked.remove(at);
            return false;
        };
        let laid = self.letter_times.poll(locator, &[need], idle);
        // Not begun (the player plays): it is asked again at the next poll.
        if self.letter_times.working() {
            self.laying = Some(self.asked.remove(at));
        }
        laid
    }

    /// Whether a statement waits or is being laid.
    pub fn working(&self) -> bool {
        self.laying.is_some() || !self.asked.is_empty()
    }
}

/// Whether a fine transcript covers the statement.
fn fine_over(kept: &[ClipTranscript], asked: &Asked) -> bool {
    kept.iter().any(|transcript| transcript.fine && transcript.from_frame <= asked.from && asked.to <= transcript.to_frame)
}

/// The words of the statement's range as the kept transcripts say them.
fn need_of(asked: &Asked, kept: &dyn Fn(&str) -> Option<Vec<ClipTranscript>>) -> Option<qnc_letter_times::Need> {
    let kept = kept(&asked.clip_id)?;
    let words: Vec<qnc_letter_times::Word> = kept
        .iter()
        .flat_map(|transcript| qnc_transcript_db::lines_cut(&transcript.lines, asked.from, asked.to, true))
        .flat_map(|line| line.words)
        .filter(|word| !word.text.trim().is_empty())
        .map(|word| qnc_letter_times::Word { from: word.from, to: word.to.max(word.from + 1), text: word.text.trim().to_string() })
        .collect();
    (!words.is_empty()).then(|| qnc_letter_times::Need { clip_id: asked.clip_id.clone(), fps: asked.fps, words })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcript(fine: bool, from: u64, to: u64) -> ClipTranscript {
        ClipTranscript { clip_id: "c".into(), from_frame: from, to_frame: to, language: "hr".into(), engine: "e".into(), lines: Vec::new(), fine }
    }

    #[test]
    fn a_statement_waits_for_its_fine_transcript() {
        let asked = Asked { clip_id: "c".into(), from: 100, to: 200, fps: 50.0 };
        assert!(!fine_over(&[transcript(false, 0, 1000)], &asked), "Parakeet's alone: not yet");
        assert!(fine_over(&[transcript(false, 0, 1000), transcript(true, 90, 210)], &asked));
    }
}
