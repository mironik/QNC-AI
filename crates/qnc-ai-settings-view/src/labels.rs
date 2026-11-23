//! The shot labels of this computer (`qnc-shot-labels-edit`): each label's name, what the model is
//! asked, whether it is a statement, new labels, the two thresholds and the caption rules; Save
//! writes the list only if the AI pieces read it.
//!
//! Every line is a row of the Settings form (`qnc_settings_rows::inline_row`: the label on the
//! left, the field, the button column), as everywhere else in QNC-AI (user 2026-11-23: "mora imati
//! isti stil kao i ostatak aplikacije"). The list of labels and the list of rules each scroll.

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_settings_window::Style;
use qnc_shot_labels_edit::{List, Rule};
use qnc_wizard_text::text;

/// What is typed for a new label, and the thresholds as typed.
#[derive(Clone, Default)]
pub struct New {
    pub name: String,
    pub prompt: String,
    pub least: Option<String>,
    pub statement_least: Option<String>,
}

/// The name a label shows: its dictionary name when its text is a key, else the text as written.
fn shown(label_text: &str) -> String {
    qnc_ui_text::host().ok().and_then(|texts| texts.get(label_text).ok()).unwrap_or(label_text).to_string()
}

/// A threshold row: the number as typed, written into `value` when it is one (0 to 0.99).
fn threshold(ui: &mut egui::Ui, width: f32, code: &str, typed: &mut Option<String>, value: &mut f32, style: &Style, theme: &Theme) {
    let mut text_now = typed.clone().unwrap_or_else(|| format!("{value:.3}"));
    qnc_settings_rows::inline_row(ui, width, &text(code), &style.metrics, theme, |ui, w| {
        ui.add(qnc_ui_kit::text_field(&mut text_now).desired_width(w.max(40.0)));
    }, |_, _| {});
    if let Ok(number) = text_now.replace(',', ".").trim().parse::<f32>() {
        *value = number.clamp(0.0, 0.99);
    }
    *typed = Some(text_now);
}

/// Draws `list` and keeps what is typed in it; gives what was said (a label added, Save).
pub fn show(ui: &mut egui::Ui, width: f32, style: &Style, theme: &Theme, list: &mut List, new: &mut New) -> Option<Result<String, String>> {
    let metrics = &style.metrics;
    let muted = |ui: &mut egui::Ui, code: &str| {
        ui.add_space(4.0);
        ui.label(RichText::new(text(code)).size(metrics.label_font_size).color(theme.text_muted));
    };
    let row = metrics.row_height + ui.spacing().item_spacing.y;
    let mut said = None;

    qnc_settings_rows::section(ui, width, &text("ai_labels.thresholds"), metrics, theme, |ui| {
        threshold(ui, width, "ai_labels.least", &mut new.least, &mut list.least, style, theme);
        threshold(ui, width, "ai_labels.statement_least", &mut new.statement_least, &mut list.statement_least, style, theme);
        muted(ui, "ai_labels.thresholds_hint");
    });

    // The two lists share the room left: the labels more, the rules the rest.
    let fixed = 3.0 * row + 6.0 * metrics.label_font_size + 120.0;
    let room = (ui.available_height() - fixed).max(6.0 * row);
    let (labels_room, rules_room) = (room * 0.6, room * 0.4);

    qnc_settings_rows::section(ui, width, &text("ai_labels.labels"), metrics, theme, |ui| {
        let mut gone = None;
        egui::ScrollArea::vertical().id_salt("qnc-ai-labels-list").max_height(labels_room).auto_shrink([false, true]).show(ui, |ui| {
            for (index, label) in list.labels.iter_mut().enumerate() {
                qnc_settings_rows::inline_row(
                    ui,
                    width - 16.0,
                    &shown(&label.text),
                    metrics,
                    theme,
                    |ui, w| {
                        ui.add(qnc_ui_kit::text_field(&mut label.prompt).desired_width(w.max(40.0))).on_hover_text(text("ai_labels.prompt_hint"));
                    },
                    |ui, _| {
                        if qnc_settings_rows::action_btn(ui, "X", theme).clicked() {
                            gone = Some(index);
                        }
                        qnc_ui_kit::check_box(ui, &mut label.statement, RichText::new(text("ai_labels.statement")).size(metrics.label_font_size).color(theme.text), true);
                    },
                );
            }
        });
        if let Some(index) = gone {
            let code = list.labels.remove(index).code;
            list.rules.retain(|rule| rule.label != code);
        }
        let mut add = false;
        qnc_settings_rows::inline_row(
            ui,
            width,
            &text("ai_labels.new_name"),
            metrics,
            theme,
            |ui, w| {
                let name_w = (w * 0.35).max(60.0);
                ui.add(qnc_ui_kit::text_field(&mut new.name).desired_width(name_w));
                ui.add(qnc_ui_kit::text_field(&mut new.prompt).desired_width((w - name_w - 8.0).max(40.0)).hint_text(text("ai_labels.new_prompt")));
            },
            |ui, _| add = qnc_settings_rows::action_btn(ui, &text("ai_labels.add"), theme).clicked(),
        );
        if add {
            match list.add(&new.name, &new.prompt) {
                Ok(()) => (new.name, new.prompt) = (String::new(), String::new()),
                Err(error) => said = Some(Err(error)),
            }
        }
        muted(ui, "ai_labels.labels_hint");
    });

    qnc_settings_rows::section(ui, width, &text("ai_labels.rules"), metrics, theme, |ui| {
        let choices: Vec<(String, String)> = list.labels.iter().filter(|label| !label.statement).map(|label| (label.code.clone(), shown(&label.text))).collect();
        let mut gone = None;
        egui::ScrollArea::vertical().id_salt("qnc-ai-labels-rules").max_height(rules_room).auto_shrink([false, true]).show(ui, |ui| {
            for (index, rule) in list.rules.iter_mut().enumerate() {
                let name = text("ai_labels.rule_n").replace("{n}", &(index + 1).to_string());
                qnc_settings_rows::inline_row(
                    ui,
                    width - 16.0,
                    &name,
                    metrics,
                    theme,
                    |ui, w| {
                        let label_w = (w * 0.35).max(60.0);
                        let current = choices.iter().find(|(code, _)| *code == rule.label).map_or_else(|| rule.label.clone(), |(_, name)| name.clone());
                        egui::ComboBox::from_id_salt(("qnc-ai-labels-rule", index)).width(label_w).selected_text(current).show_ui(ui, |ui| {
                            for (code, name) in &choices {
                                ui.selectable_value(&mut rule.label, code.clone(), name);
                            }
                        });
                        ui.add(qnc_ui_kit::text_field(&mut rule.words).desired_width((w - label_w - 16.0).max(40.0))).on_hover_text(text("ai_labels.words_hint"));
                    },
                    |ui, _| {
                        if qnc_settings_rows::action_btn(ui, "X", theme).clicked() {
                            gone = Some(index);
                        }
                    },
                );
            }
        });
        if let Some(index) = gone {
            list.rules.remove(index);
        }
        let mut add = false;
        qnc_settings_rows::trailing_actions(ui, width, metrics, |ui| add = qnc_settings_rows::action_btn(ui, &text("ai_labels.add_rule"), theme).clicked());
        if add {
            if let Some((code, _)) = choices.first() {
                list.rules.push(Rule { label: code.clone(), words: String::new() });
            }
        }
        muted(ui, "ai_labels.rules_hint");
    });
    let mut save = false;
    qnc_settings_rows::trailing_actions(ui, width, metrics, |ui| save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked());
    if save {
        said = Some(list.save().map(|()| text("machine.saved")));
        (new.least, new.statement_least) = (None, None);
    }
    said
}
