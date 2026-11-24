//! The AI of a project template (docs/102; user 2026-11-24: "u projektu AI postavke trebaju imati
//! samo izbor AI templatea, ne prikaz pojedinacnih dijelova... to imamo u AI tabu"): one row, the AI
//! template the project template takes. What each AI template switches on, and keeping or deleting
//! an own one, are the AI tab's (`qnc-ai-templates-view`); the rule and the writing are
//! `qnc-ai-features`. Drawn with the settings style block.

use qnc_ai_features::{template_of, Choice};
use qnc_settings_block::Pen;
use serde_json::Value;

/// Paints the row; true when the draft changed.
pub fn show(ui: &mut eframe::egui::Ui, pen: &Pen, choice: &mut Choice, settings: &mut Value) -> bool {
    let current = template_of(settings);
    let templates: Vec<(String, String)> = choice.templates.iter().map(|template| (template.id.clone(), Choice::template_name(template))).collect();
    let picked = pen.choice(ui, qnc_ui_text::text("ai.template_label"), "qnc_ai_template", &current, &templates);
    if let Some(id) = &picked {
        choice.choose_template(settings, id);
    }
    let errors: Vec<String> = choice.errors.iter().cloned().chain(choice.features.as_ref().err().cloned()).collect();
    if !errors.is_empty() {
        pen.said(ui, &Some(Err(errors.join("\n"))));
    }
    picked.is_some()
}
