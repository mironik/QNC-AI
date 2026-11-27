//! The script as the written form a journalist writes an OFF on (user 2026-11-27, MA Script:
//! "kolone slične pisanom obrascu"): the anchor's text boxed above, then a table of three
//! columns, the mark and time of a block on the left, the text to read in the middle, its notes
//! and the B-roll it wants on the right; under it the statements made in AI Media Assist that
//! wait for the journalist. It paints the script view with the app's theme and gives catalog
//! actions.

use eframe::egui::{self, Align, Layout, Rect, RichText, Sense, Ui, UiBuilder};
use qnc_action_event::ActionEvent;
use qnc_script::{BlockRow, Role, ScriptView, WaitingStatement, ADD_STATEMENT, PANEL, SELECT};

/// The widths of the table's left and right columns (the middle takes the rest).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Columns {
    pub left: f32,
    pub right: f32,
}

impl Columns {
    /// The columns of a table `width` wide: the left a fifth (220 to 300), the right a quarter
    /// (220 to 340).
    pub fn of(width: f32) -> Self {
        Self { left: (width * 0.2).clamp(220.0, 300.0), right: (width * 0.25).clamp(220.0, 340.0) }
    }
}

/// The inner padding of a cell.
const PAD: f32 = 6.0;

/// One row of the table: the three cells painted top down, the row as high as its highest cell,
/// `fill` behind it and the lines of the cells around it.
fn row<A, B, C>(ui: &mut Ui, theme: &qnc_desktop_theme::Theme, columns: Columns, fill: egui::Color32, outline: egui::Stroke, left: impl FnOnce(&mut Ui) -> A, middle: impl FnOnce(&mut Ui) -> B, right: impl FnOnce(&mut Ui) -> C) -> (A, B, C) {
    let width = ui.available_width();
    let top = ui.cursor().min;
    let background = ui.painter().add(egui::Shape::Noop);
    let middle_width = (width - columns.left - columns.right).max(160.0);
    let xs = [top.x, top.x + columns.left, top.x + columns.left + middle_width, top.x + width];
    let mut bottom = top.y + theme.chrome_control_height;
    let cell = |x0: f32, x1: f32, ui: &mut Ui| {
        let rect = Rect::from_min_max(egui::pos2(x0 + PAD, top.y + PAD), egui::pos2(x1 - PAD, top.y + 100_000.0));
        ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::top_down(Align::Min)))
    };
    let mut first = cell(xs[0], xs[1], ui);
    first.set_width(xs[1] - xs[0] - 2.0 * PAD);
    let a = left(&mut first);
    bottom = bottom.max(first.min_rect().bottom());
    let mut second = cell(xs[1], xs[2], ui);
    second.set_width(xs[2] - xs[1] - 2.0 * PAD);
    let b = middle(&mut second);
    bottom = bottom.max(second.min_rect().bottom());
    let mut third = cell(xs[2], xs[3], ui);
    third.set_width(xs[3] - xs[2] - 2.0 * PAD);
    let c = right(&mut third);
    bottom = bottom.max(third.min_rect().bottom());
    let rect = Rect::from_min_max(top, egui::pos2(xs[3], bottom + PAD));
    ui.painter().set(background, egui::Shape::rect_filled(rect, 0.0, fill));
    let line = egui::Stroke::new(1.0, theme.border);
    for x in [xs[1], xs[2]] {
        ui.painter().line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], line);
    }
    ui.painter().rect_stroke(rect, 0.0, outline, egui::StrokeKind::Inside);
    ui.allocate_rect(rect, Sense::hover());
    (a, b, c)
}

/// The table's head: what each column holds, on the dark of the theme.
pub fn head(ui: &mut Ui, theme: &qnc_desktop_theme::Theme, columns: Columns) {
    let title = |ui: &mut Ui, key: &'static str| {
        ui.label(RichText::new(qnc_ui_text::text(key)).color(theme.text_muted).strong().size(theme.font_ui - 1.0));
    };
    row(ui, theme, columns, theme.surface_alt, egui::Stroke::new(1.0, theme.border), |ui| title(ui, "writer.col_mark"), |ui| title(ui, "writer.col_text"), |ui| title(ui, "writer.col_notes"));
}

/// A block as a row of the table: on the left its mark, time, the caption of a statement, the
/// record of a recorded text and its buttons; in the middle its text (each part of a composite
/// statement with its timecodes and the join before it); on the right what the journalist wants
/// for B-roll and the covers suggested or accepted.
pub fn block(ui: &mut Ui, place: &'static str, block: &BlockRow, view: &ScriptView, theme: &qnc_desktop_theme::Theme, style: qnc_ui_kit::ChromeButtonStyle, columns: Columns) -> Option<ActionEvent> {
    let chosen = view.selected == Some(block.block.id);
    let outline = if chosen { egui::Stroke::new(1.5, theme.focus) } else { egui::Stroke::new(1.0, theme.border) };
    let fill = if block.is_clip() { theme.surface } else { theme.bg };
    let parts = crate::block_parts(block);
    let several = parts.len() > 1;
    let ((left_asked, left_picked), (middle_asked, middle_picked), right_asked) = row(
        ui,
        theme,
        columns,
        fill,
        outline,
        |ui| {
            let mut asked = None;
            let picked = ui.add(egui::Label::new(RichText::new(&block.mark).color(crate::mark_colour(block, theme)).strong().size(theme.font_ui + 2.0)).sense(Sense::click())).clicked();
            let time = if several { crate::parts_detail(block) } else { parts.first().map(|(time, _)| time.clone()).unwrap_or_default() };
            if !time.is_empty() {
                ui.label(RichText::new(time).color(theme.text_muted).size(theme.font_ui));
            }
            if block.is_clip() {
                let (caption, colour) = crate::caption_text(block, theme);
                ui.add(egui::Label::new(RichText::new(caption).color(colour).size(theme.font_ui)).wrap());
            }
            if let Some(event) = crate::voice_row(ui, block, view, theme, style) {
                asked = Some(event);
            }
            crate::recording_bar(ui, block, view, theme);
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                if let Some(event) = crate::block_buttons(ui, block, view, theme, style, true) {
                    asked = Some(event);
                }
            });
            (asked, picked)
        },
        |ui| {
            let mut asked = None;
            let mut picked = false;
            for (index, (time, text)) in parts.iter().enumerate() {
                if several {
                    if index > 0 {
                        ui.separator();
                        // The join before the part, while the statement is open as a SubSegment.
                        let open = view.subsegment.as_ref().filter(|open| open.block_id == block.block.id).and_then(|open| Some((open, open.parts.get(index)?)));
                        if let Some((open, part)) = open {
                            if let Some(event) = crate::part_controls::join_cell(ui, open, index, part, theme, view.ai.allows(qnc_script::JOIN_SMOOTH)) {
                                asked = Some(event);
                            }
                        }
                    }
                    ui.label(RichText::new(time).color(theme.text_muted).size(theme.font_ui - 1.0));
                }
                let (event, clicked) = crate::part_text(ui, place, block, view, theme, index, text);
                if event.is_some() {
                    asked = event;
                }
                picked |= clicked;
            }
            (asked, picked)
        },
        |ui| notes(ui, block, view, theme),
    );
    let asked = right_asked.or(middle_asked).or(left_asked);
    // The mark and the text choose the block.
    if asked.is_none() && (left_picked || middle_picked) {
        return Some(ActionEvent::key(SELECT).with_target(block.block.id.to_string()).at(PANEL));
    }
    asked
}

/// The right column of a block: the shots the journalist wants under an OFF and the covers
/// suggested or accepted; nothing for another kind.
fn notes(ui: &mut Ui, block: &BlockRow, view: &ScriptView, theme: &qnc_desktop_theme::Theme) -> Option<ActionEvent> {
    if block.role == Some(Role::Spoken) {
        crate::paint_covers(ui, block, view, theme);
    }
    None
}

/// The anchor's text (NAJAVA, ODJAVA) boxed across the width, above or under the table: its mark
/// and reading time on top with its buttons, its text under them.
pub fn anchor(ui: &mut Ui, place: &'static str, block: &BlockRow, view: &ScriptView, theme: &qnc_desktop_theme::Theme, style: qnc_ui_kit::ChromeButtonStyle) -> Option<ActionEvent> {
    let chosen = view.selected == Some(block.block.id);
    let stroke = if chosen { egui::Stroke::new(1.5, theme.focus) } else { egui::Stroke::new(1.0, theme.border) };
    let mut asked = None;
    let mut picked = false;
    egui::Frame::new().fill(theme.surface).stroke(stroke).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            let title = qnc_ui_text::fill("writer.anchor_box", &[("mark", &block.mark)]);
            picked |= ui.add(egui::Label::new(RichText::new(title).color(theme.text).strong().size(theme.font_ui + 1.0)).sense(Sense::click())).clicked();
            ui.label(RichText::new(qnc_script::seconds_label(block.seconds)).color(theme.text_muted).size(theme.font_ui));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(event) = crate::block_buttons(ui, block, view, theme, style, false) {
                    asked = Some(event);
                }
            });
        });
        ui.add_space(4.0);
        let (event, clicked) = crate::part_text(ui, place, block, view, theme, 0, &block.block.text);
        if event.is_some() {
            asked = event;
        }
        picked |= clicked;
    });
    if asked.is_none() && picked {
        return Some(ActionEvent::key(SELECT).with_target(block.block.id.to_string()).at(PANEL));
    }
    asked
}

/// The statements made in AI Media Assist that no block uses yet, under the table (user
/// 2026-11-27: they wait for the journalist in the script): a title, then a row each, its time,
/// caption and "Into the story" on the left, what is said in it in the middle, its clip on the
/// right.
pub fn waiting(ui: &mut Ui, waiting: &[WaitingStatement], theme: &qnc_desktop_theme::Theme, style: qnc_ui_kit::ChromeButtonStyle, columns: Columns) -> Option<ActionEvent> {
    if waiting.is_empty() {
        return None;
    }
    ui.add_space(10.0);
    ui.label(RichText::new(qnc_ui_text::fill("writer.waiting_title", &[("n", &waiting.len().to_string())])).color(theme.focus).strong().size(theme.font_ui + 1.0));
    ui.add_space(4.0);
    let mut asked = None;
    for statement in waiting {
        let dashed = egui::Stroke::new(1.0, theme.border);
        let (pressed, (), ()) = row(
            ui,
            theme,
            columns,
            theme.bg,
            dashed,
            |ui| {
                ui.label(RichText::new(&statement.time).color(theme.text_muted).size(theme.font_ui));
                if !statement.caption.is_empty() {
                    ui.add(egui::Label::new(RichText::new(&statement.caption).color(theme.text).size(theme.font_ui)).wrap());
                }
                ui.add_space(2.0);
                qnc_ui_kit::chrome_button(ui, qnc_ui_text::text("writer.waiting_add"), true, false, style).on_hover_text(qnc_ui_text::text("writer.waiting_add_hint")).clicked()
            },
            |ui| {
                let (shown, colour) = if statement.text.is_empty() { (qnc_ui_text::text("script.no_words").to_string(), theme.text_muted) } else { (statement.text.clone(), theme.text_muted) };
                ui.add(egui::Label::new(RichText::new(shown).color(colour).italics().size(theme.font_ui + 1.0)).wrap());
            },
            |ui| {
                ui.add(egui::Label::new(RichText::new(&statement.clip_name).color(theme.text_muted).size(theme.font_ui)).truncate());
                if statement.parts > 1 {
                    ui.label(RichText::new(qnc_ui_text::fill("writer.waiting_parts", &[("n", &statement.parts.to_string())])).color(theme.text_muted).size(theme.font_ui));
                }
            },
        );
        if pressed {
            asked = Some(ActionEvent::key(ADD_STATEMENT).with_target(statement.shot_id.clone()).at(PANEL));
        }
    }
    asked
}
