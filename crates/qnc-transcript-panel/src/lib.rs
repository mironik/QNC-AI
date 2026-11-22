//! The transcript panel (user 2026-10-26: Media Assist Audio AI, what was said in the clip on
//! the preview; a click on a word takes the playhead there). It paints the transcript view and
//! gives the click as the catalog action `transcript_cue_word` with the word's first frame; it
//! holds no state, reads nothing and knows no form.

use eframe::egui::{self, RichText, Sense};
use qnc_action_event::ActionEvent;
use qnc_source_timecode::SourceTimecode;
use qnc_transcript::{line_words, Line, TranscriptView, CUE_WORD, CUT_WORD, PANEL};

mod quotes;
mod search;

pub use search::search_field;
pub use qnc_transcript::SearchView;

pub const MODULE_ID: &str = "qnc.module.transcript-panel";

/// Paints the transcript of the shown clip on the whole of `ui`; `timecode` is the shown clip's
/// source timecode (frames are shown without it).
///
/// Only the title and the text (user 2026-11-09, AI Media Assist: "besides the name Transcript
/// no other text is needed, nor IN/OUT"): no statements, no silent sound; the IN/OUT brackets and the
/// text between IN and OUT lit whole (user 2026-11-15); at the bottom the search, how many it
/// found, Next (when there are more) and Copy of the transcript.
pub fn show(ui: &mut egui::Ui, view: &TranscriptView, theme: &qnc_desktop_theme::Theme, timecode: Option<&SourceTimecode>) -> Option<ActionEvent> {
    let rect = ui.available_rect_before_wrap();
    let bar_height = theme.font_ui + 20.0;
    let (top, bottom) = rect.split_top_bottom_at_y(rect.max.y - bar_height);
    let mut text_ui = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::top_down(egui::Align::Min)));
    let asked = paint(&mut text_ui, view, theme, timecode, Mode::Clean);
    ui.painter().rect_filled(bottom, 0.0, theme.bg);
    let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(bottom.shrink2(egui::vec2(8.0, 4.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
    let searched = search_field(&mut bar, &view.search, theme);
    let size = egui::vec2(96.0, theme.font_ui + 10.0);
    let mut next = None;
    if view.search.count > 1 && bar.add(qnc_ui_kit::sized_button(RichText::new(qnc_ui_text::text("transcript.search_next")).size(theme.font_ui), size)).clicked() {
        next = Some(ActionEvent::key(qnc_transcript::SEARCH_NEXT).at(PANEL));
    }
    if bar.add_enabled(!view.lines.is_empty(), qnc_ui_kit::sized_button(RichText::new(qnc_ui_text::text("transcript.copy")).size(theme.font_ui), size)).clicked() {
        bar.ctx().copy_text(view.lines.iter().map(|line| line.text.trim()).filter(|text| !text.is_empty()).collect::<Vec<_>>().join("\n"));
    }
    ui.allocate_rect(rect, Sense::hover());
    next.or(searched).or(asked)
}

/// The transcript of the shown clip to read and search (user 2026-11-05, MA Script media
/// overlay): the statements kept without their header row (Add IN-OUT, Copy) and without the
/// note that there are none, the words of the term typed in [`search_field`] lit.
pub fn show_searched(ui: &mut egui::Ui, view: &TranscriptView, theme: &qnc_desktop_theme::Theme, timecode: Option<&SourceTimecode>) -> Option<ActionEvent> {
    paint(ui, view, theme, timecode, Mode::Searched)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Title and text, searched (AI Media Assist).
    Clean,
    /// The statements kept below the text, searched (MA Script media overlay).
    Searched,
}

fn paint(ui: &mut egui::Ui, view: &TranscriptView, theme: &qnc_desktop_theme::Theme, timecode: Option<&SourceTimecode>, mode: Mode) -> Option<ActionEvent> {
    let statements = false;
    let clean = mode == Mode::Clean;
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, 0.0, theme.bg);
    let inner = rect.shrink(8.0);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Min)));
    ui.set_clip_rect(inner.intersect(ui.clip_rect()));
    ui.label(RichText::new(qnc_ui_text::text("transcript.title")).color(theme.text).strong());
    if !view.message.is_empty() {
        ui.label(RichText::new(&view.message).color(theme.text_muted).size(theme.font_ui));
    }
    ui.add_space(6.0);
    let quoted = if clean { None } else { quotes::show(&mut ui, view, theme, statements) };
    if !clean && !view.quotes.is_empty() {
        ui.add_space(6.0);
    }
    let early = quoted;
    let no_silences: &[(u64, u64)] = &[];
    let silences_shown = if clean { no_silences } else { &view.silences[..] };
    if view.lines.is_empty() && silences_shown.is_empty() {
        let empty = if view.clip_id.is_some() { "transcript.empty" } else { "transcript.no_clip" };
        ui.label(RichText::new(qnc_ui_text::text(empty)).color(theme.text_muted).size(theme.font_ui));
        return early;
    }
    let mut asked = None;
    let at = |frame: u64| timecode.map_or_else(|| frame.to_string(), |tc| tc.label(frame));
    // The IN and OUT brackets in AI Media Assist too: they show the double click was taken (user
    // 2026-11-15: "dodaj oznaku (zagradu) na početku i kraju selektiranog teksta").
    let ends = marked_ends(view);
    let found = Some(search::shown(ui.ctx(), &view.search));
    // New IN/OUT (a statement opened, a cut made): its text is brought into view once.
    let seen = egui::Id::new("qnc-transcript-panel-marks");
    let bring = ui.ctx().memory_mut(|memory| memory.data.get_temp::<Option<(u64, u64)>>(seen)) != Some(view.marks);
    ui.ctx().memory_mut(|memory| memory.data.insert_temp(seen, view.marks));
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(&mut ui, |ui| {
        // Lines and silences in the order of their frames.
        let mut silences = silences_shown.iter().peekable();
        // The rows the words are shown in, for the keyboard cursor (piece `qnc-text-cursor`).
        let mut rows = Vec::new();
        // By sentences, a space between them (user 2026-11-15).
        let shown = qnc_transcript::sentences(&view.lines, timecode.and_then(|tc| qnc_transcript::sentence_pause(tc.fps)));
        for line in shown.iter() {
            while let Some((from, to)) = silences.next_if(|(from, _)| *from < line.from) {
                paint_silence(ui, &at(*from), &at(*to), theme);
            }
            // The marked text is lit in AI Media Assist too, whole (user 2026-11-15: "označeni tekst
            // kompletan markiraj") and in its brackets.
            let (word, rect, line_rows) = paint_line(ui, line, view, ends, true, found.as_ref(), theme);
            rows.extend(line_rows);
            if found.as_ref().is_some_and(|found| found.bring && line_words(line).iter().any(|w| found.current.first() == Some(&(w.1, w.2)))) {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
            if bring && ends.is_some_and(|(first, _)| line_words(line).iter().any(|w| (w.1, w.2) == first)) {
                ui.scroll_to_rect(rect, Some(egui::Align::Center));
            }
            if let Some(word) = word {
                asked = Some(word);
            }
            ui.add_space(theme.font_ui * 0.8); // a space between sentences
        }
        view.rows.set(rows);
        for (from, to) in silences {
            paint_silence(ui, &at(*from), &at(*to), theme);
        }
    });
    asked.map(|(from, to, double)| match double {
        // A double click on a word is a cut there (user 2026-10-26); its first click cued it.
        true => ActionEvent::key(CUT_WORD).with_value(from).with_target(to.to_string()).at(PANEL),
        // (Its end too: with an OUT taken the click brings the OUT to it, AGENTS 0.12.)
        false => ActionEvent::key(CUE_WORD).with_value(from).with_target(to.to_string()).at(PANEL),
    }).or(early)
}

/// Sound that was transcribed and said nothing, in red (user 2026-10-26).
fn paint_silence(ui: &mut egui::Ui, from: &str, to: &str, theme: &qnc_desktop_theme::Theme) {
    let text = qnc_ui_text::fill("transcript.silence", &[("from", from), ("to", to)]);
    ui.label(RichText::new(text).color(theme.danger).size(theme.font_ui));
    ui.add_space(4.0);
}

/// One line as one text wrapped at the panel's width (user 2026-10-26: "sentences go out of the
/// panel, they should wrap"), the word said at the playhead lit; returns the first frame of a
/// clicked word.
fn paint_line(ui: &mut egui::Ui, line: &Line, view: &TranscriptView, ends: Option<((u64, u64), (u64, u64))>, show_marks: bool, found: Option<&search::Shown>, theme: &qnc_desktop_theme::Theme) -> (Option<(u64, u64, bool)>, egui::Rect, Vec<Vec<(u64, u64)>>) {
    let running = view.running.is_some_and(|(from, to)| line.from >= from && line.to <= to);
    let base = if running { theme.text_muted } else { theme.text };
    let font = egui::FontId::proportional(theme.font_ui);
    let words = line_words(line);
    let mut job = egui::text::LayoutJob::default();
    let mut spans = Vec::with_capacity(words.len());
    // The IN and OUT of the text are red brackets, the words between them lit (user 2026-11-01:
    // a statement clicked shows where its text starts and ends).
    // Close to the text, centred on its row (user 2026-11-20: they looked lost beside it).
    let bracket = |job: &mut egui::text::LayoutJob, mark: &str| {
        let mut format = egui::TextFormat::simple(egui::FontId::proportional(theme.font_ui + 4.0), theme.danger);
        format.valign = egui::Align::Center;
        job.append(mark, 1.0, format);
    };
    let is_marked = |from: u64, to: u64| show_marks && view.marks.is_some_and(|(a, b)| (a..b).contains(&((from + to) / 2)));
    // The marked text in one colour, its spaces too (user 2026-11-15): a dark half transparent
    // orange-yellow under the white letters (user 2026-11-20: grey had little contrast).
    let mark_fill = qnc_desktop_theme::text_marked();
    for (at, (text, from, to)) in words.iter().enumerate() {
        if at > 0 {
            let mut space = egui::TextFormat::simple(font.clone(), base);
            if is_marked(*from, *to) && is_marked(words[at - 1].1, words[at - 1].2) {
                space.background = mark_fill;
            }
            job.append(" ", 0.0, space);
        }
        if ends.is_some_and(|(first, _)| first == (*from, *to)) {
            bracket(&mut job, "[");
        }
        let lit = said(view, *from, *to);
        let marked = is_marked(*from, *to);
        let mut format = egui::TextFormat::simple(font.clone(), if lit { theme.black } else { base });
        let current = found.is_some_and(|found| found.current.contains(&(*from, *to)));
        let hit = found.is_some_and(|found| found.lit.contains(&(*from, *to)));
        if lit || current {
            format.color = theme.black;
            format.background = theme.focus;
        } else if hit {
            // A find of the search (user 2026-11-05).
            format.color = theme.black;
            format.background = theme.accent;
        } else if marked {
            // Between IN and OUT: what a statement takes (user 2026-10-26), lit whole (2026-11-15).
            format.color = theme.text;
            format.background = mark_fill;
        }
        let start = job.text.chars().count();
        job.append(text, 0.0, format);
        spans.push((start, start + text.chars().count(), (*from, *to)));
        if ends.is_some_and(|(_, last)| last == (*from, *to)) {
            bracket(&mut job, "]");
        }
    }
    job.wrap.max_width = ui.available_width().max(1.0);
    let galley = ui.fonts(|fonts| fonts.layout_job(job));
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    let word_at = |pos: egui::Pos2| {
        let char_index = galley.cursor_from_pos(pos - rect.min).ccursor.index;
        spans.iter().find(|(start, end, _)| (*start..=*end).contains(&char_index)).map(|(_, _, word)| *word)
    };
    if response.hovered() && response.hover_pos().and_then(word_at).is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let double = response.double_clicked();
    let clicked = if response.clicked() || double { response.interact_pointer_pos().and_then(word_at).map(|(from, to)| (from, to, double)) } else { None };
    // The word of the playhead (the keyboard cursor) is brought into view when it moves.
    let cursor = spans.iter().find(|(_, _, (from, to))| said(view, *from, *to)).map(|(start, _, _)| galley.pos_from_ccursor(egui::text::CCursor::new(*start)).translate(rect.min.to_vec2()));
    let rows = shown_rows(&galley, &spans);
    // The keyboard cursor in its gap, as in a text editor (piece `qnc-text-cursor`): before its
    // word, or after the last word of a row; half a space off the word.
    let caret = view.rows.caret().and_then(|gap| {
        spans.iter().find_map(|(start, end, (from, to))| match gap.end {
            false if *from == gap.frame => Some((*start, -1.0)),
            true if *to == gap.frame => Some((*end, 1.0)),
            _ => None,
        })
    });
    let caret = caret.map(|(at, side)| (galley.pos_from_ccursor(egui::text::CCursor::new(at)).translate(rect.min.to_vec2()), side));
    ui.painter().galley(rect.min, galley, base);
    if let Some((place, side)) = caret {
        let x = place.min.x + side * theme.font_ui * 0.15;
        ui.painter().line_segment([egui::pos2(x, place.min.y), egui::pos2(x, place.max.y)], egui::Stroke::new(2.0, theme.text));
    }
    if let Some(cursor) = cursor {
        let seen = egui::Id::new("qnc-transcript-panel-cursor");
        if ui.ctx().memory_mut(|memory| memory.data.get_temp::<Option<u64>>(seen)) != Some(view.playhead) {
            ui.ctx().memory_mut(|memory| memory.data.insert_temp(seen, view.playhead));
            ui.scroll_to_rect(cursor, None);
        }
    }
    (clicked, rect, rows)
}

/// The words of a laid out line by the rows they are shown in, top to bottom.
fn shown_rows(galley: &egui::Galley, spans: &[(usize, usize, (u64, u64))]) -> Vec<Vec<(u64, u64)>> {
    let mut rows: Vec<(f32, Vec<(u64, u64)>)> = Vec::new();
    for (start, _, word) in spans {
        let top = galley.pos_from_ccursor(egui::text::CCursor::new(*start)).min.y;
        match rows.last_mut() {
            Some((row_top, words)) if (*row_top - top).abs() < 1.0 => words.push(*word),
            _ => rows.push((top, vec![*word])),
        }
    }
    rows.into_iter().map(|(_, words)| words).collect()
}

/// The words of a line with their frames (a line without words is one word).
/// The first and the last word between IN and OUT, by the middle of the word.
fn marked_ends(view: &TranscriptView) -> Option<((u64, u64), (u64, u64))> {
    let (a, b) = view.marks?;
    let mut inside = view.lines.iter().flat_map(line_words).map(|(_, from, to)| (from, to)).filter(|(from, to)| (a..b).contains(&((from + to) / 2)));
    let first = inside.next()?;
    Some((first, inside.last().unwrap_or(first)))
}

fn said(view: &TranscriptView, from: u64, to: u64) -> bool {
    view.playhead.is_some_and(|playhead| (from..to).contains(&playhead))
}

#[cfg(test)]
mod tests {
    use super::*;
    use qnc_transcript::Word;

    #[test]
    fn the_in_and_out_words_are_the_first_and_last_between_the_marks_across_lines() {
        let line = |words: &[(u64, u64)]| Line {
            from: words[0].0,
            to: words[words.len() - 1].1,
            text: String::new(),
            words: words.iter().map(|&(from, to)| Word { from, to, text: "w".into() }).collect(),
        };
        let mut view = TranscriptView { lines: std::sync::Arc::new(vec![line(&[(0, 10), (10, 20)]), line(&[(30, 40), (40, 50)])]), ..TranscriptView::default() };
        assert_eq!(marked_ends(&view), None, "no IN/OUT, no brackets");
        view.marks = Some((12, 46));
        assert_eq!(marked_ends(&view), Some(((10, 20), (40, 50))));
        view.marks = Some((31, 39));
        assert_eq!(marked_ends(&view), Some(((30, 40), (30, 40))), "one word is both");
    }
}
