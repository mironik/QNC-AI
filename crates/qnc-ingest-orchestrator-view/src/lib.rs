//! The choice of the ingest orchestrator of each form, in the settings (user 2026-11-16:
//! "orkestratori se biraju u postavkama"): per form (the Ingest form, the tray) the orchestrators
//! of the catalog that serve it, the one chosen on this computer selected. A choice is written
//! at once (`qnc-ingest-orchestrator`); what is shown is read again from it.

use std::path::Path;

use eframe::egui;
use qnc_ingest_orchestrator::{choose, chosen, for_form, INGEST, TRAY};
use qnc_wizard_text::text;

/// Draws the choice; a line under it says what went wrong, if anything.
pub fn show(ui: &mut egui::Ui, root: &Path) {
    ui.label(egui::RichText::new(text("orchestrator.title")).strong());
    for form in [INGEST, TRAY] {
        let options = match for_form(root, form) {
            Ok(options) => options,
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
                continue;
            }
        };
        let now = chosen(root, form).ok().map(|orchestrator| orchestrator.id);
        let shown = now.as_deref().and_then(|id| options.iter().find(|option| option.id == id)).map_or(String::new(), |option| text(&option.label));
        ui.horizontal(|ui| {
            ui.label(text(&format!("orchestrator.form.{form}")));
            egui::ComboBox::from_id_salt(("qnc-ingest-orchestrator", form)).selected_text(shown).show_ui(ui, |ui| {
                for option in &options {
                    if ui.selectable_label(now.as_deref() == Some(option.id.as_str()), text(&option.label)).clicked() {
                        if let Err(error) = choose(root, form, &option.id) {
                            ui.colored_label(ui.visuals().error_fg_color, error);
                        }
                    }
                }
            });
        });
    }
}
