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
    for bar in progress.bars() {
        ui.horizontal(|ui| {
            ui.add_sized([110.0, 20.0], egui::Label::new(RichText::new(text(bar.text)).color(ink)));
            let label = if bar.stopped {
                fill("card_ingest.stopped", &[("done", &bar.done.to_string()), ("clips", &progress.clips.to_string())])
            } else {
                format!("{} / {}", bar.done, progress.clips)
            };
            ui.add(egui::ProgressBar::new(bar.done.min(total) as f32 / total as f32).desired_width(260.0).text(label));
            let running = !bar.stopped && bar.done < progress.clips;
            let x = ui.add_enabled(running, egui::Button::new("X").min_size(egui::vec2(22.0, 20.0)));
            if x.on_hover_text(text("card_ingest.stop_hint")).clicked() {
                stop = Some(bar.function);
            }
            if let Some(time) = time_of(&bar) {
                ui.label(RichText::new(clock(time)).color(ink));
            }
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
