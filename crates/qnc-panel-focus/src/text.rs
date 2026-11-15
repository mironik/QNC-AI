use super::*;
use qnc_text_cursor::TextStep;

impl PanelFocus {
    /// The keyboard on the transcript text (user 2026-11-15: "u tekst panelu nemamo kursora"):
    /// a click on the text puts it there (the source timeline's keys go on as they are); the
    /// arrows move the cursor (piece `qnc-text-cursor`, the same for any text of timed words)
    /// and the playhead to its word, Shift with the arrows selects the text, its IN/OUT on the
    /// source timeline. `None`: not the text's.
    pub(crate) fn text_key(&mut self, event: &ActionEvent, preview: &mut SourcePreview, timeline: &mut TimelineProjection) -> Option<bool> {
        if event.is_at(qnc_transcript::PANEL) {
            self.panel = Panel::SourceTimeline;
            self.on_text = true;
            if let Some(word) = event.unsigned().zip(event.target.as_deref().and_then(|to| to.parse().ok())) {
                self.text_cursor.place(word, timeline.playhead_frame);
                self.text_cursor.moved();
            }
            return None;
        }
        if event.at.is_some() || self.panel != Panel::SourceTimeline {
            self.on_text = false;
            return None;
        }
        if !self.on_text || self.taken(timeline) {
            return None;
        }
        let (step, select) = match event.action_id.as_str() {
            "step_back_frame" => (TextStep::Left, false),
            "step_forward_frame" => (TextStep::Right, false),
            "step_prev_part" => (TextStep::Up, false),
            "step_next_part" => (TextStep::Down, false),
            "transcript_select_left" => (TextStep::Left, true),
            "transcript_select_right" => (TextStep::Right, true),
            "transcript_select_up" => (TextStep::Up, true),
            "transcript_select_down" => (TextStep::Down, true),
            _ => return None,
        };
        // The text's words by lines, as the transcript shows them.
        let lines: Vec<Vec<qnc_text_cursor::WordSpan>> = self.side.views().transcript.lines.iter().map(|line| line.words.iter().map(|word| (word.from, word.to)).collect()).collect();
        let Some((to, selection)) = self.text_cursor.step(&lines, timeline.playhead_frame, step, select) else { return Some(false) };
        if let Some((first, last)) = selection {
            self.source.select_words(timeline, first, last);
        }
        Some(self.source.cue(&TimelineIntent::CueFrame(to.0), preview) || selection.is_some())
    }
}
