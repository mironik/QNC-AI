//! The choice of the ingest orchestrator of each form, in the settings (user 2026-11-16:
//! "orkestratori se biraju u postavkama"; "polja orkestratora nisu stilski usklađena s ostatkom
//! forme"): drawn with the settings' own rows (`qnc-settings-rows`): a title like the other groups,
//! per form (the Ingest form, the tray) its label in the label column and the orchestrators of the
//! catalog that serve it in the field column, the one chosen on this computer selected. A choice is
//! written at once (`qnc-ingest-orchestrator`); what is shown is read again from it.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_ingest_orchestrator::{choose, chosen, for_form, INGEST, TRAY};
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;

/// Draws the choice `width` wide, in the rows of the settings panel.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    ui.add_space(metrics.section_gap);
    ui.label(RichText::new(text("orchestrator.title").to_uppercase()).size(metrics.label_font_size).strong().color(theme.text_muted));
    for form in [INGEST, TRAY] {
        let options = for_form(root, form);
        let now = chosen(root, form).ok().map(|orchestrator| orchestrator.id);
        let mut error = None;
        qnc_settings_rows::inline_row(
            ui,
            width,
            &text(&format!("orchestrator.form.{form}")),
            metrics,
            theme,
            |ui, field_width| match &options {
                Ok(options) => {
                    let shown = now.as_deref().and_then(|id| options.iter().find(|option| option.id == id)).map_or(String::new(), |option| text(&option.label));
                    egui::ComboBox::from_id_salt(("qnc-ingest-orchestrator", form)).width(field_width).selected_text(shown).show_ui(ui, |ui| {
                        for option in options {
                            if ui.selectable_label(now.as_deref() == Some(option.id.as_str()), text(&option.label)).clicked() {
                                if let Err(failed) = choose(root, form, &option.id) {
                                    error = Some(failed);
                                }
                            }
                        }
                    });
                }
                Err(failed) => {
                    ui.colored_label(theme.danger, failed);
                }
            },
            |_, _| {},
        );
        if let Some(error) = error {
            ui.colored_label(theme.danger, error);
        }
    }
}
