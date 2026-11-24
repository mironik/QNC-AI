//! A border between two places the user drags to make them wider or narrower (user 2026-11-14:
//! "da se paneli mogu širiti po korisnikovim potrebama, a ne da budu fiksni"). It only reports
//! how far it was dragged; the caller keeps the sizes.

use eframe::egui::{self, Rect};

/// How wide the grip of a border is on each side of its line.
const GRIP: f32 = 4.0;

/// A border along `line` (a vertical border when `across` is false: dragged left and right; a
/// horizontal one when it is true: dragged up and down); the pixels it was dragged this frame.
/// It is drawn as a line of `color` (the border of the theme), of `held` (the focus) while the
/// pointer is on it or drags it, with the resize cursor (user 2026-11-14: the borders were not
/// seen).
pub fn drag_edge(ui: &mut egui::Ui, id: egui::Id, line: Rect, across: bool, color: egui::Color32, held: egui::Color32) -> f32 {
    let grip = if across { line.expand2(egui::vec2(0.0, GRIP)) } else { line.expand2(egui::vec2(GRIP, 0.0)) };
    let response = ui.interact(grip, id, egui::Sense::drag());
    let active = response.hovered() || response.dragged();
    if active {
        ui.ctx().set_cursor_icon(if across { egui::CursorIcon::ResizeVertical } else { egui::CursorIcon::ResizeHorizontal });
    }
    let (from, to) = if across { (line.left_center(), line.right_center()) } else { (line.center_top(), line.center_bottom()) };
    ui.painter().line_segment([from, to], egui::Stroke::new(if active { 3.0 } else { 2.0 }, if active { held } else { color }));
    if response.dragged() {
        let delta = response.drag_delta();
        if across { delta.y } else { delta.x }
    } else {
        0.0
    }
}
