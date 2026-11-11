//! QNC-AI's import form (user 2026-11-11: the tray starts Ingest when a card of the list is in;
//! "kod mora biti odvojen od GUI"). It shows the clips of the cards the tray read into their
//! temporary JSON and ticks the shots of the story. All it does is in `qnc-card-pick`; this form
//! only lays out and draws what that block gives, and sends the clicks back. One copy runs.

#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};

use eframe::egui;
use qnc_card_pick::CardPick;
use qnc_desktop_theme::{ShellLayoutContract, Theme};
use qnc_media_card::{CardGridAction, CardMetrics, CardStyle, MediaCardFeatures, PoolGrid, PoolLook, StatusDotsMode, Ticks};
use qnc_wizard_text::{fill, text};
use serde::Deserialize;

#[derive(Deserialize)]
struct Layout {
    media_card: Metrics,
    bar_height: f32,
    button_height: f32,
}

#[derive(Deserialize)]
struct Metrics {
    min_card_width: f32,
    card_text_height: f32,
    grid_gap: f32,
}

fn read<T: for<'a> Deserialize<'a>>(root: &Path, name: &str) -> Result<T, String> {
    let path = root.join("contracts/ui").join(name);
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

struct Form {
    pick: CardPick,
    layout: Layout,
    theme: Theme,
}

impl eframe::App for Form {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pick.poll();
        qnc_desktop_theme::apply(ctx, &self.theme);
        let theme = &self.theme;
        egui::TopBottomPanel::top("bar").exact_height(self.layout.bar_height).frame(egui::Frame::new().fill(theme.panel).inner_margin(8.0)).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let clips = self.pick.rows().len().to_string();
                let ticked = self.pick.ticked().to_string();
                let cards = self.pick.cards().len().to_string();
                ui.label(fill("ingest.status", &[("cards", &cards), ("clips", &clips), ("ticked", &ticked)]));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let style = qnc_ui_kit::ChromeButtonStyle { text: theme.text, border: theme.border, focus: theme.focus, height: self.layout.button_height };
                    // The next step (project and import) comes in W3.
                    let _ = qnc_ui_kit::strong_button(ui, &text("ingest.next"), false, theme.accent, self.layout.button_height);
                    if qnc_ui_kit::chrome_button(ui, &text("ingest.tick_none"), self.pick.ticked() > 0, false, style).clicked() {
                        self.pick.tick_none();
                    }
                    if qnc_ui_kit::chrome_button(ui, &text("ingest.tick_all"), !self.pick.rows().is_empty(), false, style).clicked() {
                        self.pick.tick_all();
                    }
                });
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.surface).inner_margin(8.0)).show(ctx, |ui| {
            let metrics = &self.layout.media_card;
            let look = PoolLook {
                style: card_style(theme),
                metrics: CardMetrics { min_card_width: metrics.min_card_width, card_text_height: metrics.card_text_height, grid_gap: metrics.grid_gap },
                features: MediaCardFeatures { ticks: Ticks::Selected, top_right_marker: false, status_dots: StatusDotsMode::Off, remove_chosen: None },
                imported_marker: theme.accent,
                not_imported_marker: theme.text_muted,
                saving: "",
                save_failed: "",
                focus: theme.focus,
            };
            let empty = text("ingest.empty");
            let grid = PoolGrid { rect: ui.available_rect_before_wrap(), chosen_id: "", panel_focused: false, step: None, empty_message: &empty, id_salt: "card-pick" };
            let rows: Vec<_> = self.pick.rows().iter().collect();
            match qnc_media_card::show_pool(ui, &rows, &grid, &look) {
                Some(CardGridAction::ToggleSelection(id)) | Some(CardGridAction::Activate(id)) => self.pick.toggle(&id),
                _ => {}
            }
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

/// The colours of a card, from the theme.
fn card_style(theme: &Theme) -> CardStyle {
    let rgba = egui::Color32::from_rgba_unmultiplied;
    CardStyle {
        raised: theme.surface_alt,
        surface: theme.surface,
        border: theme.border,
        text: theme.text,
        muted: theme.text_muted,
        select_red: theme.danger,
        dot_ready: egui::Color32::from_rgb(0x30, 0xd1, 0x58),
        dot_original_ready: egui::Color32::from_rgb(0x0a, 0x84, 0xff),
        dot_pending: egui::Color32::from_rgb(0xff, 0xd6, 0x0a),
        dot_error: egui::Color32::from_rgb(0xff, 0x45, 0x3a),
        check_on: egui::Color32::from_rgb(0xff, 0x95, 0x00),
        check_mark: egui::Color32::from_rgb(0x1a, 0x1a, 0x1a),
        check_off: rgba(0, 0, 0, 90),
        check_off_rim: rgba(255, 255, 255, 140),
        remove_fill: rgba(0, 0, 0, 120),
        remove_hover: theme.danger.gamma_multiply(0.85),
        remove_mark: rgba(255, 255, 255, 220),
    }
}

fn main() {
    qnc_dev_diagnostics::log_panics("qnc-ai-ingest");
    let _one = match qnc_tray::OneCopy::take("QNC-AI-ingest") {
        Ok(Some(one)) => one,
        _ => return,
    };
    let root: PathBuf = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let form = (|| -> Result<Form, String> {
        let shell: ShellLayoutContract = read(&root, "shell.layout.json")?;
        Ok(Form { pick: CardPick::new(&root), layout: read(&root, "card-pick.layout.json")?, theme: Theme::from_contract(&shell) })
    })();
    let form = match form {
        Ok(form) => form,
        Err(error) => {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-ingest not started: {error}"));
            return;
        }
    };
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_title(text("ingest.title")).with_maximized(true), ..Default::default() };
    let _ = eframe::run_native("qnc-ai-ingest", options, Box::new(|_| Ok(Box::new(form))));
}
