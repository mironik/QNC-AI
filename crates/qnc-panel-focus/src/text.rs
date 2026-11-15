use super::*;
use qnc_text_cursor::{Gap, TextStep};

impl PanelFocus {
    /// The keyboard on the transcript text (user 2026-11-15: "u tekst panelu nemamo kursora";
    /// the panel is in the Tab walk): a click on the text or Tab puts it there; the cursor
    /// stands in a gap between words (piece `qnc-text-cursor`, the same for any text of timed
    /// words); the arrows move it and the playhead to it, Shift with the arrows selects the words
    /// between, their IN/OUT on the source timeline; the other keys are the source timeline's.
    /// `None`: not the text's; `Some`: the answer of the route.
    pub(crate) fn text_key(&mut self, event: &ActionEvent, pieces: (&mut SourcePreview, &mut WrapSession, &mut ProgramSegments, &mut TimelineProjection)) -> Option<Option<bool>> {
        if event.is_at(qnc_transcript::PANEL) {
            self.panel = Panel::Transcript;
            // A click on a word: the cursor before it.
            if let Some(from) = event.unsigned() {
                self.text_cursor.place(Gap { frame: from, end: false }, pieces.3.playhead_frame);
                self.text_cursor.moved();
            }
            return None;
        }
        if self.panel != Panel::Transcript || event.at.is_some() || matches!(event.action_id.as_str(), "focus_next" | "focus_prev") {
            return None;
        }
        let step = match event.action_id.as_str() {
            "step_back_frame" => Some((TextStep::Left, false)),
            "step_forward_frame" => Some((TextStep::Right, false)),
            "step_prev_part" => Some((TextStep::Up, false)),
            "step_next_part" => Some((TextStep::Down, false)),
            "transcript_select_left" => Some((TextStep::Left, true)),
            "transcript_select_right" => Some((TextStep::Right, true)),
            "transcript_select_up" => Some((TextStep::Up, true)),
            "transcript_select_down" => Some((TextStep::Down, true)),
            _ => None,
        };
        match step {
            // A taken IN/OUT keeps the arrows (as on the source timeline).
            Some((step, select)) if !self.taken(pieces.3) => {
                let rows = self.text_rows();
                let Some((to, selection)) = self.text_cursor.step(&rows, pieces.3.playhead_frame, step, select) else { return Some(Some(false)) };
                if let Some((from, till)) = selection {
                    self.source.select_words(pieces.3, (from, from), (till, till));
                }
                self.side.views().transcript.rows.set_caret(Some(to));
                Some(Some(self.source.cue(&TimelineIntent::CueFrame(to.frame), pieces.0) || selection.is_some()))
            }
            // The other keys of the text are the source timeline's; the keyboard stays on the text.
            _ => {
                self.panel = Panel::SourceTimeline;
                let done = self.route(event.clone(), pieces);
                if self.panel == Panel::SourceTimeline {
                    self.panel = Panel::Transcript;
                }
                Some(done)
            }
        }
    }

    /// The text's words by the rows the panel shows them in (a line wraps into several, user
    /// 2026-11-15: the cursor skipped rows); before a paint, its lines.
    fn text_rows(&self) -> Vec<Vec<qnc_text_cursor::WordSpan>> {
        let view = &self.side.views().transcript;
        let shown = view.rows.get();
        if shown.is_empty() {
            view.lines.iter().map(|line| line.words.iter().map(|word| (word.from, word.to)).collect()).collect()
        } else {
            shown
        }
    }

    /// The cursor the transcript panel draws: while the keyboard is on the text, its gap (it
    /// follows the playhead when that goes elsewhere); none otherwise.
    pub(crate) fn show_text_caret(&self, timeline: &TimelineProjection) {
        let caret = (self.panel == Panel::Transcript).then(|| self.text_cursor.gap(&self.text_rows(), timeline.playhead_frame)).flatten();
        self.side.views().transcript.rows.set_caret(caret);
    }
}
