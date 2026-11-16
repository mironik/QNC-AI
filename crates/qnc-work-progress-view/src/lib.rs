//! The bars of the background work as the wizard's import window shows them (user 2026-11-12:
//! "progres bar po funkciji"; the X beside a bar stops the function when the user does not need
//! it): one row per function the project has on, its name, its bar with done of all, the X and the
//! time. Used by the wizard and by AI Media Assist (user 2026-11-16: "treba izgled kao wizard, da
//! korisnik može isključiti neke korake koje ne treba"). It draws and says which X was pressed;
//! the caller stops the function.

use std::time::Duration;

use eframe::egui::{self, Color32, RichText};
use qnc_work_progress::{Bar, Progress};
use qnc_wizard_text::{fill, text};

/// Draws the bars of `progress` in colour `ink`; `time_of` gives the time shown beside a bar (how
/// long its function took, or runs). Gives the function whose X was pressed.
pub fn bars(ui: &mut egui::Ui, progress: &Progress, ink: Color32, time_of: &dyn Fn(&Bar) -> Option<Duration>) -> Option<&'static str> {
    let mut stop = None;
    let total = progress.clips.max(1);
    // The bar fills the width between the name and the X with the time, so its right edge is the
    // one of the fields under it (user 2026-11-16: "ingest nema sređene margine").
    const NAME: f32 = 110.0;
    const X: f32 = 22.0;
    const TIME: f32 = 52.0;
    for bar in progress.bars() {
        ui.horizontal(|ui| {
            let gap = ui.spacing().item_spacing.x;
            // The name on the same left edge as everything else (user 2026-11-16: it was centred).
            ui.allocate_ui_with_layout(egui::vec2(NAME, 20.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_min_width(NAME);
                ui.label(RichText::new(text(bar.text)).color(ink));
            });
            let label = if bar.stopped {
                fill("card_ingest.stopped", &[("done", &bar.done.to_string()), ("clips", &progress.clips.to_string())])
            } else {
                format!("{} / {}", bar.done, progress.clips)
            };
            let width = (ui.available_width() - X - TIME - gap * 2.0).max(80.0);
            ui.add(egui::ProgressBar::new(bar.done.min(total) as f32 / total as f32).desired_width(width).text(label));
            let running = !bar.stopped && bar.done < progress.clips;
            let x = ui.add_enabled(running, qnc_ui_kit::sized_button("X", egui::vec2(X, 20.0)));
            if x.on_hover_text(text("card_ingest.stop_hint")).clicked() {
                stop = Some(bar.function);
            }
            let time = time_of(&bar).map(clock).unwrap_or_default();
            ui.allocate_ui_with_layout(egui::vec2(TIME, 20.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(time).color(ink));
            });
        });
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
