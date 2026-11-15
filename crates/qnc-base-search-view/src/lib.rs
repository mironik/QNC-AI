//! The archive search as an overlay window over any QNC-AI window (user 2026-11-15: "pretragu bi
//! dodao kao samostalnu kocku koja bi se pozivala u obliku overlay prozora"). It draws and keeps
//! only what is typed and what was found; the search is `qnc-base-search`'s.

use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};
use qnc_base_search::{Hit, Search};
use qnc_wizard_text::{fill, text};

/// The overlay: open or not, what is asked and what was found.
#[derive(Default)]
pub struct SearchOverlay {
    open: bool,
    query: String,
    asked: Option<String>,
    search: Search,
    hits: Vec<Hit>,
    error: Option<String>,
    focus: bool,
}

impl SearchOverlay {
    pub fn open(&mut self) {
        (self.open, self.focus) = (true, true);
        self.asked = None;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Draws the overlay over `ctx` while open; the bases of `root` are searched as the user types.
    pub fn show(&mut self, ctx: &egui::Context, root: &Path) {
        if !self.open {
            return;
        }
        if let Some(found) = self.search.poll() {
            match found {
                Ok(hits) => (self.hits, self.error) = (hits, None),
                Err(error) => self.error = Some(error),
            }
        }
        if self.asked.as_deref() != Some(self.query.as_str()) && !self.search.busy() {
            self.asked = Some(self.query.clone());
            self.search.start(&PathBuf::from(root), &self.query);
        }
        if self.search.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        let screen = ctx.screen_rect();
        let size = egui::vec2((screen.width() * 0.6).clamp(420.0, 820.0), (screen.height() * 0.7).clamp(300.0, 720.0));
        let mut open = true;
        egui::Window::new(RichText::new(text("search.title")).strong())
            .id(egui::Id::new("qnc-base-search"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                let field = ui.add(egui::TextEdit::singleline(&mut self.query).hint_text(text("search.hint")).desired_width(f32::INFINITY));
                if std::mem::take(&mut self.focus) {
                    field.request_focus();
                }
                ui.add_space(6.0);
                if let Some(error) = &self.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
                let weak = ui.visuals().weak_text_color();
                ui.label(RichText::new(match self.hits.len() {
                    0 => text("search.none"),
                    count => fill("search.found", &[("count", &count.to_string())]),
                }).color(weak));
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for hit in &self.hits {
                        let base = &hit.base;
                        ui.label(RichText::new(base.shown()).strong());
                        if !base.location.is_empty() {
                            ui.label(RichText::new(fill("search.place", &[("place", &base.location)])).color(weak));
                        }
                        if !base.description.is_empty() {
                            ui.label(&base.description);
                        }
                        if !hit.lines.is_empty() {
                            ui.label(RichText::new(hit.lines.join(" · ")).italics());
                        }
                        ui.add_space(4.0);
                        ui.separator();
                    }
                });
            });
        if !open || ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.open = false;
        }
    }
}
