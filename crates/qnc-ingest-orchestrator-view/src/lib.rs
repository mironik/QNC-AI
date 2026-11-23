//! The choice of the ingest orchestrator of each form, in the settings (user 2026-11-16:
//! "orkestratori se biraju u postavkama"; "polja orkestratora nisu stilski usklađena s ostatkom
//! forme"): drawn by the settings style block (`qnc_settings_block::Pen`): a title like the other
//! groups, per form (the Ingest form, the tray) a row with the orchestrators of the catalog that
//! serve it, the one chosen on this computer selected. A choice is written at once
//! (`qnc-ingest-orchestrator`); what is shown is read again from it.

use std::path::Path;

use eframe::egui;
use qnc_ingest_orchestrator::{choose, chosen, for_form, INGEST, TRAY};
use qnc_settings_block::Pen;
use qnc_wizard_text::text;

/// Draws the choice with the settings style block's pen.
pub fn show(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    pen.title(ui, "orchestrator.title", "");
    for form in [INGEST, TRAY] {
        let label = text(&format!("orchestrator.form.{form}"));
        match for_form(root, form) {
            Ok(options) => {
                let now = chosen(root, form).ok().map(|orchestrator| orchestrator.id).unwrap_or_default();
                let choices: Vec<(String, String)> = options.iter().map(|option| (option.id.clone(), text(&option.label))).collect();
                if let Some(id) = pen.choice(ui, &label, ("qnc-ingest-orchestrator", form), &now, &choices) {
                    if let Err(failed) = choose(root, form, &id) {
                        pen.said(ui, &Some(Err(failed)));
                    }
                }
            }
            Err(failed) => pen.row(ui, &label, |ui, _| pen.said(ui, &Some(Err(failed.clone()))), |_, _| {}),
        }
    }
}
