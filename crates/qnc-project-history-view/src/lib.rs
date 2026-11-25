//! The window of a project's history over the form (user 2026-11-25: Ctrl+Shift+H): the moments
//! newest first (when, who, which piece, what), who owns the project, a row lit, and going back
//! to the moment of the lit row. Passive paint only: what it shows comes from
//! `qnc-project-history`, a click gives back the catalog action and its row.

use eframe::egui::{self, Align2, RichText, ScrollArea, Stroke, Vec2};
use qnc_project_history::{HistoryView, OPEN, RESTORE};

/// The action that lights a row (a click; its value is the row).
pub const LIGHT: &str = "project_history_light";

const WIDTH: f32 = 760.0;
const LIST_HEIGHT: f32 = 420.0;

/// Shows the window in the middle of the screen over everything; the action clicked and its row.
pub fn show(ctx: &egui::Context, theme: &qnc_desktop_theme::Theme, view: &HistoryView) -> Option<(&'static str, Option<i64>)> {
    let mut asked = None;
    egui::Area::new(egui::Id::new("qnc_project_history"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::NONE.fill(theme.panel).stroke(Stroke::new(2.0, theme.focus)).corner_radius(6.0).inner_margin(16.0).show(ui, |ui| {
                ui.set_width(WIDTH);
                ui.label(RichText::new(qnc_ui_text::text("history.title")).color(theme.text).size(theme.font_ui + 4.0).strong());
                ui.label(RichText::new(&view.owner).color(theme.text_muted).size(theme.font_ui));
                ui.add_space(8.0);
                let lit_id = egui::Id::new("qnc_project_history_lit");
                let shown_lit = ui.ctx().memory(|memory| memory.data.get_temp::<usize>(lit_id));
                ScrollArea::vertical().max_height(LIST_HEIGHT).auto_shrink([false, true]).show(ui, |ui| {
                    for (at, row) in view.rows.iter().enumerate() {
                        let lit = at == view.lit;
                        let text = RichText::new(row).size(theme.font_ui).color(if lit { theme.focus } else { theme.text });
                        let response = ui.add(egui::Label::new(if lit { text.strong() } else { text }).sense(egui::Sense::click()));
                        if lit && shown_lit != Some(at) {
                            response.scroll_to_me(None);
                        }
                        if response.double_clicked() {
                            asked = Some((RESTORE, Some(at as i64)));
                        } else if response.clicked() {
                            asked = Some((LIGHT, Some(at as i64)));
                        }
                    }
                });
                ui.ctx().memory_mut(|memory| memory.data.insert_temp(lit_id, view.lit));
                if !view.message.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new(&view.message).color(theme.text).size(theme.font_ui));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let height = theme.chrome_control_height;
                    if qnc_ui_kit::strong_button(ui, qnc_ui_text::text("history.go_back"), view.may && !view.working, theme.focus, height).clicked() {
                        asked = Some((RESTORE, Some(view.lit as i64)));
                    }
                    if qnc_ui_kit::small_button(ui, qnc_ui_text::text("history.close"), true, false).clicked() {
                        asked = Some((OPEN, None));
                    }
                });
            });
        });
    asked
}
