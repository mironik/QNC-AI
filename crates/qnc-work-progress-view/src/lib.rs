//! The bars of the background work as the wizard's import window shows them (user 2026-11-12:
//! "progres bar po funkciji"; the X beside a bar stops the function when the user does not need
//! it): one row per function the project has on, its name, its bar with done of all, the X and the
//! time. Used by the wizard and by AI Media Assist (user 2026-11-16: "treba izgled kao wizard, da
//! korisnik može isključiti neke korake koje ne treba"). It draws and says which X was pressed;
//! the caller stops the function.

use std::time::Duration;

use eframe::egui::{self, Color32};
use qnc_work_progress::{Bar, Progress};
use qnc_wizard_text::{fill, text};

/// The measures of the bars: a row's height, the name, count, time and X columns, the gap between
/// columns and the bar's thickness (user 2026-11-16: the form's look file gives them).
#[derive(Clone, Copy, Debug)]
pub struct BarsLook {
    pub row: f32,
    pub name: f32,
    pub count: f32,
    pub time: f32,
    pub x: f32,
    pub gap: f32,
    pub bar: f32,
}

impl Default for BarsLook {
    fn default() -> Self {
        Self { row: 26.0, name: 120.0, count: 78.0, time: 50.0, x: 22.0, gap: 12.0, bar: 6.0 }
    }
}

/// Draws the bars of `progress` in colour `ink`; `time_of` gives the time shown beside a bar (how
/// long its function took, or runs). Gives the function whose X was pressed.
pub fn bars(ui: &mut egui::Ui, progress: &Progress, ink: Color32, time_of: &dyn Fn(&Bar) -> Option<Duration>) -> Option<&'static str> {
    bars_with(ui, progress, ink, time_of, &BarsLook::default())
}

/// As [`bars`], with the measures of `look`.
pub fn bars_with(ui: &mut egui::Ui, progress: &Progress, ink: Color32, time_of: &dyn Fn(&Bar) -> Option<Duration>, look: &BarsLook) -> Option<&'static str> {
    // Every row is laid out in fixed columns from the same left and right edge (user 2026-11-16:
    // "dizajn ti je očajan": the X and the time slid right row by row): the name, a thin round bar
    // filling the width left, the count, the time, and a quiet X while the function runs.
    #[allow(non_snake_case)]
    let BarsLook { row: ROW, name: NAME, count: COUNT, time: TIME, x: X, gap: GAP, bar: BAR } = *look;
    let mut stop = None;
    let total = progress.clips.max(1) as f32;
    let (muted, track, done_fill) = (ui.visuals().weak_text_color(), ui.visuals().extreme_bg_color, ui.visuals().selection.bg_fill);
    let font = egui::TextStyle::Body.resolve(ui.style());
    let width = ui.available_width();
    // The X column is there only while some function runs; otherwise the time ends at the right
    // edge (user 2026-11-16: "donje i desne margine").
    let running = |bar: &Bar| !bar.stopped && bar.done < progress.clips;
    let any_running = progress.bars().iter().any(running);
    for bar in progress.bars() {
        let (row, _) = ui.allocate_exact_size(egui::vec2(width, ROW), egui::Sense::hover());
        let painter = ui.painter_at(row);
        let mid = row.center().y;
        let x_rect = egui::Rect::from_center_size(egui::pos2(row.right() - X / 2.0, mid), egui::vec2(X, X));
        let time_right = if any_running { x_rect.left() - GAP } else { row.right() };
        let count_right = time_right - TIME - GAP;
        let bar_left = row.left() + NAME + GAP;
        let bar_right = (count_right - COUNT - GAP).max(bar_left + 40.0);
        painter.text(egui::pos2(row.left(), mid), egui::Align2::LEFT_CENTER, text(bar.text), font.clone(), ink);
        let track_rect = egui::Rect::from_min_max(egui::pos2(bar_left, mid - BAR / 2.0), egui::pos2(bar_right, mid + BAR / 2.0));
        painter.rect_filled(track_rect, BAR / 2.0, track);
        let share = (bar.done as f32 / total).clamp(0.0, 1.0);
        if share > 0.0 {
            let filled = egui::Rect::from_min_max(track_rect.min, egui::pos2(track_rect.left() + track_rect.width() * share, track_rect.bottom()));
            painter.rect_filled(filled, BAR / 2.0, if bar.stopped { muted } else { done_fill });
        }
        let count = if bar.stopped {
            fill("card_ingest.stopped", &[("done", &bar.done.to_string()), ("clips", &progress.clips.to_string())])
        } else {
            format!("{} / {}", bar.done, progress.clips)
        };
        painter.text(egui::pos2(count_right, mid), egui::Align2::RIGHT_CENTER, count, font.clone(), ink);
        if let Some(time) = time_of(&bar) {
            painter.text(egui::pos2(time_right, mid), egui::Align2::RIGHT_CENTER, clock(time), font.clone(), muted);
        }
        if running(&bar) {
            let x = ui.interact(x_rect, ui.id().with(("qnc-work-stop", bar.function)), egui::Sense::click()).on_hover_text(text("card_ingest.stop_hint"));
            let colour = if x.hovered() { ink } else { muted };
            if x.hovered() {
                painter.rect_filled(x_rect, 4.0, track);
            }
            painter.text(x_rect.center(), egui::Align2::CENTER_CENTER, "×", font.clone(), colour);
            if x.clicked() {
                stop = Some(bar.function);
            }
        }
    }
    stop
}

/// A time as m:ss (h:mm:ss from an hour on).
pub fn clock(time: Duration) -> String {
    let seconds = time.as_secs();
    match seconds / 3600 {
        0 => format!("{}:{:02}", seconds / 60, seconds % 60),
        hours => format!("{hours}:{:02}:{:02}", seconds / 60 % 60, seconds % 60),
    }
}

/// The bars with the measures of a form's look (`bar_row`, `bar_name`, `bar_count`, `bar_time`,
/// `bar_x`, `bar_col_gap`, `bar_thickness`), each column of the first row marked so it can be
/// moved on the form (user 2026-11-16).
pub fn bars_marked(ui: &mut egui::Ui, progress: &Progress, ink: Color32, time_of: &dyn Fn(&Bar) -> Option<Duration>, look: &qnc_form_look::FormLook) -> Option<&'static str> {
    use qnc_form_look::Grows;
    let measures = BarsLook {
        row: look.get("bar_row"),
        name: look.get("bar_name"),
        count: look.get("bar_count"),
        time: look.get("bar_time"),
        x: look.get("bar_x"),
        gap: look.get("bar_col_gap"),
        bar: look.get("bar_thickness"),
    };
    let (left, right, top) = (ui.cursor().left(), ui.cursor().left() + ui.available_width(), ui.cursor().top());
    let stop = bars_with(ui, progress, ink, time_of, &measures);
    if look.editing() && !progress.bars().is_empty() {
        let BarsLook { row, name, count, time, x, gap, bar } = measures;
        let any_running = progress.bars().iter().any(|bar| !bar.stopped && bar.done < progress.clips);
        let x_left = right - x;
        let time_right = if any_running { x_left - gap } else { right };
        let count_right = time_right - time - gap;
        let mid = top + row / 2.0;
        let span = |from: f32, to: f32| egui::Rect::from_x_y_ranges(from..=to, top..=top + row);
        look.mark(ui, span(left, left + name), "bar_name", Grows::Right);
        look.mark(ui, span(left + name, left + name + gap), "bar_col_gap", Grows::Right);
        look.mark(ui, egui::Rect::from_x_y_ranges(left + name + gap..=count_right - count - gap, mid - bar / 2.0..=mid + bar / 2.0), "bar_thickness", Grows::Down);
        look.mark(ui, span(count_right - count, count_right), "bar_count", Grows::Left);
        look.mark(ui, span(time_right - time, time_right), "bar_time", Grows::Left);
        if any_running {
            look.mark(ui, span(x_left, right), "bar_x", Grows::Left);
        }
        look.mark(ui, egui::Rect::from_x_y_ranges(left..=right, top + row - 3.0..=top + row), "bar_row", Grows::Down);
    }
    stop
}
