//! What every block of QNC-AI's Settings form draws alike (user 2026-11-23: "forma postavke se
//! slaže od kocaka"): a section's title with its hint, a typed field in a settings row, and what a
//! write said. Each block reads and writes its own setting; the form only puts the blocks in the
//! order its layout names.

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;

/// A section's title (upper case) and its hint, both by their text codes.
pub fn title(ui: &mut egui::Ui, code: &str, hint: &str, metrics: &PanelMetrics, theme: &Theme) {
    ui.add_space(metrics.section_gap);
    ui.label(RichText::new(text(code).to_uppercase()).size(metrics.label_font_size).strong().color(theme.text_muted));
    ui.label(RichText::new(text(hint)).size(metrics.label_font_size).color(theme.text_muted));
}

/// What the last write said: done in the muted colour, an error in the danger colour.
pub fn said(ui: &mut egui::Ui, said: &Option<Result<String, String>>, theme: &Theme) {
    match said {
        Some(Ok(message)) => {
            ui.colored_label(theme.text_muted, message);
        }
        Some(Err(error)) => {
            ui.colored_label(theme.danger, error);
        }
        None => {}
    }
}

/// A typed field in a settings row, its label by its text code; `secret` hides what is typed.
pub fn field(ui: &mut egui::Ui, width: f32, label: &str, value: &mut String, metrics: &PanelMetrics, theme: &Theme, secret: bool) {
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text(label),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(value).desired_width(field_width.max(40.0)).password(secret));
        },
        |_, _| {},
    );
}
