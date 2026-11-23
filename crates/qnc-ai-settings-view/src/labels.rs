//! The shot labels of this computer (`qnc-shot-labels-edit`): each label's name, what the model is
//! asked, whether it is a statement, new labels, the two thresholds and the caption rules; Save
//! writes the list only if the AI pieces read it.
//!
//! Everything is drawn by the Settings form's style piece (`qnc-settings-form`: the label above its
//! field, the window's grid), so the alignment, the room between the rows and the font are those of
//! every other setting (user 2026-11-23: "isto poravnanje i isti razmak između redova, i isti font").
//! The list of labels and the list of rules each scroll on their own.

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_settings_form::{FieldKind, FieldSpec, OptionSets};
use qnc_settings_window::Style;
use qnc_shot_labels_edit::{List, Rule};
use qnc_wizard_text::text;
use serde_json::Value;

/// What is typed for a new label.
#[derive(Clone, Default)]
pub struct New {
    pub name: String,
    pub prompt: String,
}

/// The name a label shows: its dictionary name when its text is a key, else the text as written.
fn shown(label_text: &str) -> String {
    qnc_ui_text::host().ok().and_then(|texts| texts.get(label_text).ok()).unwrap_or(label_text).to_string()
}

fn spec(kind: FieldKind, label: String, id: String, hint: String) -> FieldSpec {
    FieldSpec { kind, label, path: "v".into(), id, options: None, default: String::new(), hint }
}

/// One field in the style's grid, `width` wide; gives its text after drawing.
fn cell(ui: &mut egui::Ui, width: f32, field: &FieldSpec, value: Value, style: &Style) -> String {
    let mut settings = serde_json::json!({ "v": value });
    qnc_settings_form::field_grid(ui, width, &mut settings, std::slice::from_ref(field), &OptionSets::new(), &style.grid);
    match settings.get("v") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

/// What sits beside a field, level with the field (below the label's line).
fn beside(ui: &mut egui::Ui, style: &Style, add: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.add_space(style.grid.label_size + 10.0);
        ui.horizontal(add);
    });
}

/// Draws `list` and keeps what is typed in it; gives what was said (a label added, Save).
pub fn show(ui: &mut egui::Ui, width: f32, style: &Style, theme: &Theme, list: &mut List, new: &mut New) -> Option<Result<String, String>> {
    let size = style.grid.label_size;
    let muted = |ui: &mut egui::Ui, code: &str| {
        ui.label(RichText::new(text(code)).size(size).color(theme.text_muted));
    };
    let row = style.grid.cell_height + style.grid.gap;
    let side = 150.0;
    let field = (width - side - style.grid.gap).max(style.grid.min_cell_width);
    let mut said = None;

    qnc_settings_rows::section(ui, width, &text("ai_labels.thresholds"), &style.metrics, theme, |ui| {
        let half = (width - style.grid.gap) / 2.0;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = style.grid.gap;
            for (code, value) in [("ai_labels.least", &mut list.least), ("ai_labels.statement_least", &mut list.statement_least)] {
                let typed = cell(ui, half, &spec(FieldKind::Number, text(code), format!("qnc-ai-labels-{code}"), String::new()), Value::from(f64::from(*value)), style);
                if let Ok(number) = typed.replace(',', ".").parse::<f32>() {
                    *value = number.clamp(0.0, 0.99);
                }
            }
        });
        muted(ui, "ai_labels.thresholds_hint");
    });

    // The two lists share what room is left: the labels more, the rules the rest.
    let fixed = 3.0 * row + 140.0;
    let room = (ui.available_height() - fixed).max(4.0 * row);
    let (labels_room, rules_room) = (room * 0.55, room * 0.45);

    qnc_settings_rows::section(ui, width, &text("ai_labels.labels"), &style.metrics, theme, |ui| {
        let mut gone = None;
        egui::ScrollArea::vertical().id_salt("qnc-ai-labels-list").max_height(labels_room).auto_shrink([false, true]).show(ui, |ui| {
            for (index, label) in list.labels.iter_mut().enumerate() {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = style.grid.gap;
                    let field_spec = spec(FieldKind::Text, shown(&label.text), format!("qnc-ai-label-{}", label.code), text("ai_labels.prompt_hint"));
                    label.prompt = cell(ui, field, &field_spec, Value::from(label.prompt.clone()), style);
                    beside(ui, style, |ui| {
                        qnc_ui_kit::check_box(ui, &mut label.statement, RichText::new(text("ai_labels.statement")).size(size).color(theme.text), true);
                        if qnc_settings_rows::action_btn(ui, "X", theme).clicked() {
                            gone = Some(index);
                        }
                    });
                });
            }
        });
        if let Some(index) = gone {
            let code = list.labels.remove(index).code;
            list.rules.retain(|rule| rule.label != code);
        }
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = style.grid.gap;
            let name_w = (field * 0.35).max(style.grid.min_cell_width);
            new.name = cell(ui, name_w, &spec(FieldKind::Text, text("ai_labels.new_name"), "qnc-ai-labels-new-name".into(), String::new()), Value::from(new.name.clone()), style);
            new.prompt = cell(ui, field - name_w - style.grid.gap, &spec(FieldKind::Text, text("ai_labels.new_prompt"), "qnc-ai-labels-new-prompt".into(), text("ai_labels.prompt_hint")), Value::from(new.prompt.clone()), style);
            beside(ui, style, |ui| {
                if qnc_settings_rows::action_btn(ui, &text("ai_labels.add"), theme).clicked() {
                    match list.add(&new.name, &new.prompt) {
                        Ok(()) => *new = New::default(),
                        Err(error) => said = Some(Err(error)),
                    }
                }
            });
        });
        muted(ui, "ai_labels.labels_hint");
    });

    qnc_settings_rows::section(ui, width, &text("ai_labels.rules"), &style.metrics, theme, |ui| {
        let choices: Vec<(String, String)> = list.labels.iter().filter(|label| !label.statement).map(|label| (label.code.clone(), shown(&label.text))).collect();
        let label_w = (field * 0.35).max(style.grid.min_cell_width);
        let mut gone = None;
        egui::ScrollArea::vertical().id_salt("qnc-ai-labels-rules").max_height(rules_room).auto_shrink([false, true]).show(ui, |ui| {
            for (index, rule) in list.rules.iter_mut().enumerate() {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = style.grid.gap;
                    if let Some(code) = qnc_settings_form::choice_cell(ui, label_w, &text("ai_labels.rule_label"), &format!("qnc-ai-labels-rule-{index}"), &rule.label, &choices, &style.grid) {
                        rule.label = code;
                    }
                    let words = spec(FieldKind::Text, text("ai_labels.rule_words"), format!("qnc-ai-labels-rule-words-{index}"), text("ai_labels.words_hint"));
                    rule.words = cell(ui, field - label_w - style.grid.gap, &words, Value::from(rule.words.clone()), style);
                    beside(ui, style, |ui| {
                        if qnc_settings_rows::action_btn(ui, "X", theme).clicked() {
                            gone = Some(index);
                        }
                    });
                });
            }
        });
        if let Some(index) = gone {
            list.rules.remove(index);
        }
        if qnc_settings_rows::action_btn(ui, &text("ai_labels.add_rule"), theme).clicked() {
            if let Some((code, _)) = choices.first() {
                list.rules.push(Rule { label: code.clone(), words: String::new() });
            }
        }
        muted(ui, "ai_labels.rules_hint");
    });
    if qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked() {
        said = Some(list.save().map(|()| text("machine.saved")));
    }
    said
}
