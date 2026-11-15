//! A window brought above the other applications (user 2026-11-13: the card's window when the
//! ingest ends; 2026-11-15: "projekt izbornik se otvori minimiziran, pa se ne vidi"): not
//! minimised, on top, focused and asking for attention, until the user is in it (a click, a key)
//! or goes to another window; then it is an ordinary window again. Windows does not let a
//! program started in the background take the focus, so it stays on top until then.

use eframe::egui;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub struct WindowFront {
    since: Option<Instant>,
}

impl WindowFront {
    /// Brings the window above the others now.
    pub fn bring(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
        self.since = Some(Instant::now());
    }

    /// Every frame: once the user is in it, or has gone to another window (after a moment), it is
    /// an ordinary window again.
    pub fn follow(&mut self, ctx: &egui::Context) {
        let Some(since) = self.since else { return };
        let (in_it, away) = ctx.input(|input| (input.pointer.any_pressed() || (input.focused && !input.keys_down.is_empty()), input.viewport().focused == Some(false)));
        if in_it || (away && since.elapsed() >= Duration::from_secs(1)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Reset));
            self.since = None;
        }
    }

    /// Whether it is held above the others.
    pub fn is_on(&self) -> bool {
        self.since.is_some()
    }
}
