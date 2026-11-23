//! The shot labels of this computer (`qnc-shot-labels-edit`): each label's name, what the model is
//! asked, whether it is a statement, new labels, the two thresholds and the caption rules; Save
//! writes the list only if the AI pieces read it.

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_settings_window::Style;
use qnc_shot_labels_edit::{List, Rule};
use qnc_wizard_text::text;

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

/// Draws `list` and keeps what is typed in it; gives what was said (a label added, Save).
pub fn show(ui: &mut egui::Ui, width: f32, style: &Style, theme: &Theme, list: &mut List, new: &mut New) -> Option<Result<String, String>> {
    let size = style.grid.label_size;
    let muted = |ui: &mut egui::Ui, code: &str| {
        ui.label(RichText::new(text(code)).size(size).color(theme.text_muted));
    };
    let mut said = None;
    let field = (width - 220.0).max(120.0);
    qnc_settings_rows::section(ui, width, &text("ai_labels.thresholds"), &style.metrics, theme, |ui| {
        for (code, value) in [("ai_labels.least", &mut list.least), ("ai_labels.statement_least", &mut list.statement_least)] {
            ui.horizontal(|ui| {
                ui.add_sized([200.0, style.grid.cell_height], egui::Label::new(RichText::new(text(code)).size(size).color(theme.text)));
                ui.add(egui::DragValue::new(value).range(0.0..=0.99).speed(0.001).max_decimals(3));
            });
        }
        muted(ui, "ai_labels.thresholds_hint");
    });
    qnc_settings_rows::section(ui, width, &text("ai_labels.labels"), &style.metrics, theme, |ui| {
        let mut gone = None;
        for (index, label) in list.labels.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let name = shown(&label.text);
                if name == label.text {
                    ui.add(qnc_ui_kit::text_field(&mut label.text).desired_width(160.0));
                } else {
                    ui.add_sized([160.0, style.grid.cell_height], egui::Label::new(RichText::new(name).size(size).color(theme.text)).truncate());
                }
                ui.add(qnc_ui_kit::text_field(&mut label.prompt).desired_width(field - 160.0)).on_hover_text(text("ai_labels.prompt_hint"));
                qnc_ui_kit::check_box(ui, &mut label.statement, RichText::new(text("ai_labels.statement")).size(size).color(theme.text), true);
                if qnc_settings_rows::action_btn(ui, "X", theme).clicked() {
                    gone = Some(index);
                }
            });
        }
        if let Some(index) = gone {
            let code = list.labels.remove(index).code;
            list.rules.retain(|rule| rule.label != code);
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add(qnc_ui_kit::text_field(&mut new.name).desired_width(160.0).hint_text(text("ai_labels.new_name")));
            ui.add(qnc_ui_kit::text_field(&mut new.prompt).desired_width(field - 160.0).hint_text(text("ai_labels.new_prompt")));
            if qnc_settings_rows::action_btn(ui, &text("ai_labels.add"), theme).clicked() {
                match list.add(&new.name, &new.prompt) {
                    Ok(()) => *new = New::default(),
                    Err(error) => said = Some(Err(error)),
                }
            }
        });
        muted(ui, "ai_labels.labels_hint");
    });
    qnc_settings_rows::section(ui, width, &text("ai_labels.rules"), &style.metrics, theme, |ui| {
        let choices: Vec<(String, String)> = list.labels.iter().filter(|label| !label.statement).map(|label| (label.code.clone(), shown(&label.text))).collect();
        let mut gone = None;
        for (index, rule) in list.rules.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                let current = choices.iter().find(|(code, _)| *code == rule.label).map_or_else(|| rule.label.clone(), |(_, name)| name.clone());
                egui::ComboBox::from_id_salt(("qnc-ai-labels-rule", index)).width(160.0).selected_text(current).show_ui(ui, |ui| {
                    for (code, name) in &choices {
                        ui.selectable_value(&mut rule.label, code.clone(), name);
                    }
                });
                ui.add(qnc_ui_kit::text_field(&mut rule.words).desired_width(field - 160.0)).on_hover_text(text("ai_labels.words_hint"));
                if qnc_settings_rows::action_btn(ui, "X", theme).clicked() {
                    gone = Some(index);
                }
            });
        }
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
