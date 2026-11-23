//! The shot labels of this computer (`qnc-shot-labels-edit`): each label's name, what the model is
//! asked, whether it is a statement, new labels, the two thresholds and the caption rules; Save
//! writes the list only if the AI pieces read it. Drawn by the settings style block
//! (`qnc_settings_block::Pen`): rows, and the labels and the rules each in a list that scrolls.

use eframe::egui;
use qnc_settings_block::Pen;
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
fn threshold(ui: &mut egui::Ui, pen: &Pen, code: &str, typed: &mut Option<String>, value: &mut f32) {
    let mut now = typed.clone().unwrap_or_else(|| format!("{value:.3}"));
    pen.text(ui, code, &mut now, "");
    if let Ok(number) = now.replace(',', ".").trim().parse::<f32>() {
        *value = number.clamp(0.0, 0.99);
    }
    *typed = Some(now);
}

/// Draws `list` and keeps what is typed in it; gives what was said (a label added, Save).
pub fn show(ui: &mut egui::Ui, pen: &Pen, list: &mut List, new: &mut New) -> Option<Result<String, String>> {
    let mut said = None;
    pen.section(ui, &text("ai_labels.thresholds"), |ui| {
        threshold(ui, pen, "ai_labels.least", &mut new.least, &mut list.least);
        threshold(ui, pen, "ai_labels.statement_least", &mut new.statement_least, &mut list.statement_least);
        pen.hint(ui, &text("ai_labels.thresholds_hint"));
    });

    // The two lists share the whole rows left: the labels more, the rules the rest.
    let rows_left = (ui.available_height() / pen.rows_height(ui, 1.0) - 10.0).floor().max(6.0);
    let labels_rows = (rows_left * 0.6).floor();

    pen.section(ui, &text("ai_labels.labels"), |ui| {
        let mut gone = None;
        pen.list(ui, "qnc-ai-labels-list", labels_rows, |ui, pen| {
            for (index, label) in list.labels.iter_mut().enumerate() {
                pen.row(
                    ui,
                    &shown(&label.text),
                    |ui, row| {
                        row.input(ui, &mut label.prompt, row.width, "");
                    },
                    |ui, buttons| {
                        if buttons.action(ui, "X") {
                            gone = Some(index);
                        }
                        buttons.check(ui, &mut label.statement, &text("ai_labels.statement"), pen.metrics);
                    },
                );
            }
        });
        if let Some(index) = gone {
            let code = list.labels.remove(index).code;
            list.rules.retain(|rule| rule.label != code);
        }
        let mut add = false;
        pen.listed().row(
            ui,
            &text("ai_labels.new_name"),
            |ui, row| {
                let name_w = row.width * 0.35;
                row.input(ui, &mut new.name, name_w, "");
                row.input(ui, &mut new.prompt, row.width - name_w - row.gap(ui), &text("ai_labels.new_prompt"));
            },
            |ui, buttons| add = buttons.action(ui, &text("ai_labels.add")),
        );
        if add {
            match list.add(&new.name, &new.prompt) {
                Ok(()) => (new.name, new.prompt) = (String::new(), String::new()),
                Err(error) => said = Some(Err(error)),
            }
        }
        pen.hint(ui, &text("ai_labels.labels_hint"));
    });

    pen.section(ui, &text("ai_labels.rules"), |ui| {
        let choices: Vec<(String, String)> = list.labels.iter().filter(|label| !label.statement).map(|label| (label.code.clone(), shown(&label.text))).collect();
        let mut gone = None;
        pen.list(ui, "qnc-ai-labels-rules", rows_left - labels_rows, |ui, pen| {
            for (index, rule) in list.rules.iter_mut().enumerate() {
                pen.row(
                    ui,
                    &text("ai_labels.rule_n").replace("{n}", &(index + 1).to_string()),
                    |ui, row| {
                        let label_w = row.width * 0.35;
                        if let Some(code) = row.choice(ui, ("qnc-ai-labels-rule", index), label_w, &rule.label, &choices) {
                            rule.label = code;
                        }
                        row.input(ui, &mut rule.words, row.width - label_w - row.gap(ui), &text("ai_labels.words_hint"));
                    },
                    |ui, buttons| {
                        if buttons.action(ui, "X") {
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
        pen.listed().buttons(ui, |ui, buttons| add = buttons.action(ui, &text("ai_labels.add_rule")));
        if add {
            if let Some((code, _)) = choices.first() {
                list.rules.push(Rule { label: code.clone(), words: String::new() });
            }
        }
        pen.hint(ui, &text("ai_labels.rules_hint"));
    });
    let mut save = false;
    pen.listed().buttons(ui, |ui, buttons| save = buttons.primary(ui, &text("machine.save"), true));
    if save {
        said = Some(list.save().map(|()| text("machine.saved")));
        (new.least, new.statement_least) = (None, None);
    }
    said
}
