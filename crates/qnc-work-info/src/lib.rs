//! The background work of the active project in a window's header (user 2026-11-16: "ne prikazuj
//! u polju prijepisa fajlove, možeš kreirati info prozor kao što ima wizard"; "treba izgled kao
//! wizard, da korisnik može isključiti neke korake"; "dodaj dugme minimize"; "minimizirani wizard
//! premjesti u header forme"). Open, it is a box under the header, at the right: how many clips,
//! one bar per function the project has on, each with the X that stops it. Minimized, it is a chip
//! in the header with how far the work is in all; a click opens it. Gone once everything is done.
//! Whether it is minimized is a choice of this window's view, not kept anywhere.

use std::path::{Path, PathBuf};
use std::time::Duration;

use eframe::egui::{self, Rect, RichText, Sense};
use qnc_wizard_text::{fill, text};
use qnc_work_progress::{Follower, Progress};

/// How wide the open box is.
const WIDTH: f32 = 470.0;
/// How wide the chip in the header is.
const CHIP: f32 = 230.0;

pub struct WorkInfo {
    root: PathBuf,
    follower: Option<Follower>,
    minimized: bool,
}

impl WorkInfo {
    pub fn new(root: &Path) -> Self {
        // It opens minimized, a chip in the header (user 2026-11-19: "Rad u pozadini otvaraj
        // minimiziran"); a click opens it.
        Self { root: root.to_path_buf(), follower: None, minimized: true }
    }

    /// What is done now, while something is left to do.
    fn running(&mut self) -> Option<Progress> {
        let follower = self.follower.get_or_insert_with(|| Follower::start(&self.root));
        follower.poll();
        follower.last().copied().filter(|work| work.clips > 0 && !work.complete())
    }

    /// The chip in the header, at the right of `space` (when minimized); reads what is done.
    /// Gives how wide it drew.
    pub fn header(&mut self, ui: &mut egui::Ui, space: Rect) -> f32 {
        let Some(progress) = self.running() else { return 0.0 };
        ui.ctx().request_repaint_after(Duration::from_millis(500));
        if !self.minimized {
            return 0.0;
        }
        let rect = Rect::from_min_max(egui::pos2(space.right() - CHIP, space.top() + 5.0), egui::pos2(space.right() - 8.0, space.bottom() - 5.0));
        let response = ui.interact(rect, egui::Id::new("qnc-work-info-chip"), Sense::click()).on_hover_text(text("work.restore"));
        let visuals = ui.visuals().clone();
        let fill_colour = if response.hovered() { visuals.widgets.hovered.weak_bg_fill } else { visuals.widgets.inactive.weak_bg_fill };
        ui.painter().rect_filled(rect, 3.0, fill_colour);
        let share = share_done(&progress);
        let line = format!("{}  {:.0} %", text("work.title"), share * 100.0);
        ui.painter().text(rect.left_center() + egui::vec2(8.0, -3.0), egui::Align2::LEFT_CENTER, line, egui::FontId::proportional(13.0), visuals.text_color());
        let bar = Rect::from_min_max(egui::pos2(rect.left() + 8.0, rect.bottom() - 6.0), egui::pos2(rect.right() - 8.0, rect.bottom() - 3.0));
        ui.painter().rect_filled(bar, 1.0, visuals.extreme_bg_color);
        ui.painter().rect_filled(Rect::from_min_max(bar.min, egui::pos2(bar.left() + bar.width() * share, bar.bottom())), 1.0, visuals.selection.bg_fill);
        if response.clicked() {
            self.minimized = false;
        }
        CHIP
    }

    /// The open box under the header (`top`, the header's bottom), at the window's right.
    pub fn overlay(&mut self, ctx: &egui::Context, top: f32) {
        if self.minimized {
            return;
        }
        let Some(progress) = self.follower.as_ref().and_then(Follower::last).copied().filter(|work| work.clips > 0 && !work.complete()) else { return };
        let right = ctx.screen_rect().right();
        let mut stop = None;
        let mut minimize = false;
        egui::Area::new(egui::Id::new("qnc-work-info"))
            .order(egui::Order::Foreground)
            .fixed_pos(egui::pos2(right - WIDTH - 8.0, top + 8.0))
            .show(ctx, |ui| {
                let visuals = ui.visuals().clone();
                egui::Frame::new()
                    .fill(visuals.panel_fill)
                    .stroke(visuals.widgets.noninteractive.bg_stroke)
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(WIDTH - 24.0);
                        let ink = visuals.text_color();
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(text("work.title")).color(ink).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                minimize = ui.add(qnc_ui_kit::sized_button("_", egui::vec2(22.0, 20.0))).on_hover_text(text("work.minimize")).clicked();
                            });
                        });
                        ui.label(RichText::new(fill("card_ingest.clips", &[("clips", &progress.clips.to_string())])).color(ink));
                        ui.add_space(8.0);
                        stop = qnc_work_progress_view::bars(ui, &progress, ink, &|_| None);
                        if progress.failed > 0 {
                            ui.add_space(6.0);
                            ui.label(RichText::new(fill("card_ingest.failed", &[("failed", &progress.failed.to_string())])).color(visuals.error_fg_color));
                        }
                    });
            });
        if minimize {
            self.minimized = true;
        }
        if let Some(function) = stop {
            qnc_work_progress::stop(&self.root, function);
        }
    }
}

/// How much of the work is done in all: of every bar the project has on, the clips done (a stopped
/// function counts as done).
pub fn share_done(progress: &Progress) -> f32 {
    let bars = progress.bars();
    let all = (bars.len() * progress.clips).max(1);
    let done: usize = bars.iter().map(|bar| if bar.stopped { progress.clips } else { bar.done.min(progress.clips) }).sum();
    done as f32 / all as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_share_done_counts_a_stopped_function_as_done() {
        let mut progress = Progress { clips: 4, imported: 4, posters: 2, ..Default::default() };
        assert!((share_done(&progress) - 0.75).abs() < 1e-6);
        progress.stopped = 1 << 1; // posters, second of qnc_stopped_work::ALL
        assert!((share_done(&progress) - 1.0).abs() < 1e-6);
    }
}
