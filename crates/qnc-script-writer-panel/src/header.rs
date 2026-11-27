//! The header of the story over the writing form (user 2026-11-24: before writing, "overlay
//! obrazac za naziv, novinar, snimatelj... u jednom stupcu"): the fields of the story sheet one
//! under the other, each in one row, several names in the same row (user 2026-11-27: "sve samo po jedan red"); Enter
//! goes to the next field, at the last one it opens the writing; Esc closes it, nothing kept.
//! Over the writing form the header kept is shown in several columns. It paints only and gives
//! the actions of `qnc-story-header`.

use eframe::egui::{self, Id, RichText, Ui};
use qnc_action_event::ActionEvent;
use qnc_field_spec::FieldKind;
use qnc_script::PANEL;
use qnc_story_header::{HeaderView, CANCEL, CONFIRM, TEXT};

const WIDTH: f32 = 560.0;
const LABEL: f32 = 170.0;
/// The header kept, over the writing form: this many fields side by side.
pub const COLUMNS: usize = 3;

fn field_id(path: &str, index: usize) -> Id {
    Id::new(("qnc_story_header_field", path, index))
}

/// The overlay while it is open; its answer.
pub fn overlay(ctx: &egui::Context, view: &HeaderView, theme: &qnc_desktop_theme::Theme) -> Option<ActionEvent> {
    if !view.open {
        return None;
    }
    let mut asked = None;
    // The form under it is dimmed and takes no clicks.
    let screen = ctx.screen_rect();
    egui::Area::new(Id::new("qnc_story_header_dim")).order(egui::Order::Middle).fixed_pos(screen.min).show(ctx, |ui| {
        ui.painter().rect_filled(screen, 0.0, egui::Color32::from_black_alpha(150));
        ui.allocate_rect(screen, egui::Sense::click());
    });
    let style = qnc_ui_kit::ChromeButtonStyle { text: theme.text, border: theme.border, focus: theme.focus, height: theme.chrome_control_height };
    egui::Area::new(Id::new("qnc_story_header")).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(ctx, |ui| {
        egui::Frame::new().fill(theme.surface).stroke(egui::Stroke::new(1.0, theme.focus)).corner_radius(6.0).inner_margin(20.0).show(ui, |ui| {
            ui.set_width(WIDTH);
            ui.label(RichText::new(qnc_ui_text::text("header.title")).color(theme.text).strong().size(theme.font_ui + 5.0));
            ui.label(RichText::new(&view.sheet_name).color(theme.text_muted).size(theme.font_ui));
            ui.add_space(10.0);
            // The fields in order; Enter goes from one typed row to the next.
            let mut order: Vec<Id> = Vec::new();
            // The row Enter goes to: its place in `order` (past the end: the end of the form).
            let mut next: Option<usize> = None;
            for field in &view.fields {
                ui.horizontal_top(|ui| {
                    ui.add_sized([LABEL, theme.chrome_control_height], egui::Label::new(RichText::new(&field.label).color(theme.text_muted).size(theme.font_ui)));
                    ui.vertical(|ui| match field.kind {
                        Some(FieldKind::Choice) => {
                            let shown = field.choices.iter().find(|(code, _)| *code == field.value).map_or(field.value.clone(), |(_, name)| name.clone());
                            egui::ComboBox::from_id_salt(field_id(&field.path, 0)).width(ui.available_width()).selected_text(shown).show_ui(ui, |ui| {
                                for (code, name) in &field.choices {
                                    if ui.selectable_label(*code == field.value, name).clicked() {
                                        asked = Some(ActionEvent::key(TEXT).with_target(format!("{}\u{1f}{code}", field.path)).at(PANEL));
                                    }
                                }
                            });
                        }
                        _ => {
                            let mut text = field.value.clone();
                            let id = field_id(&field.path, 0);
                            let edit = ui.add(qnc_ui_kit::text_field(&mut text).id(id).hint_text(&field.hint).desired_width(ui.available_width()));
                            order.push(id);
                            if edit.changed() {
                                asked = Some(ActionEvent::key(TEXT).with_target(format!("{}\u{1f}{text}", field.path)).at(PANEL));
                            }
                            if edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                                next = Some(order.len());
                            }
                        }
                    });
                });
                ui.add_space(4.0);
            }
            // The keyboard is in the first field when the overlay opens.
            let opened = Id::new("qnc_story_header_opened");
            if !ui.ctx().data(|data| data.get_temp::<bool>(opened).unwrap_or(false)) {
                if let Some(first) = order.first() {
                    ui.ctx().memory_mut(|memory| memory.request_focus(*first));
                }
                ui.ctx().data_mut(|data| data.insert_temp(opened, true));
            }
            match next {
                Some(at) if at < order.len() => ui.ctx().memory_mut(|memory| memory.request_focus(order[at])),
                Some(_) => asked = Some(ActionEvent::key(CONFIRM).at(PANEL)),
                None => {}
            }
            if !view.message.is_empty() {
                ui.label(RichText::new(&view.message).color(theme.danger).size(theme.font_ui));
            }
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if qnc_ui_kit::chrome_button(ui, qnc_ui_text::text("header.write"), true, true, style).clicked() {
                    asked = Some(ActionEvent::key(CONFIRM).at(PANEL));
                }
                if qnc_ui_kit::chrome_button(ui, qnc_ui_text::text("header.cancel"), true, false, style).clicked() {
                    asked = Some(ActionEvent::key(CANCEL).at(PANEL));
                }
            });
        });
    });
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        asked = Some(ActionEvent::key(CANCEL).at(PANEL));
    }
    if asked.as_ref().is_some_and(|event| event.action_id == CONFIRM || event.action_id == CANCEL) {
        ctx.data_mut(|data| data.remove::<bool>(Id::new("qnc_story_header_opened")));
    }
    asked
}

/// The header kept, in `COLUMNS` columns of name and value, under the title of the writing form;
/// how many rows it took.
pub fn shown(ui: &mut Ui, view: &HeaderView, theme: &qnc_desktop_theme::Theme) {
    if view.shown.is_empty() {
        return;
    }
    egui::Grid::new("qnc_story_header_shown").num_columns(COLUMNS * 2).spacing([10.0, 2.0]).show(ui, |ui| {
        for (index, (label, value)) in view.shown.iter().enumerate() {
            ui.label(RichText::new(label).color(theme.text_muted).size(theme.font_ui - 1.0));
            ui.label(RichText::new(value).color(theme.text).size(theme.font_ui - 1.0));
            if (index + 1) % COLUMNS == 0 {
                ui.end_row();
            }
        }
    });
}

/// The rows the header kept takes in `COLUMNS` columns.
pub fn shown_rows(view: &HeaderView) -> usize {
    view.shown.len().div_ceil(COLUMNS)
}
