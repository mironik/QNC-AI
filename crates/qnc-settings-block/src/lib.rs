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

/// Fields (`qnc-settings-form`'s field specs) as rows of the Settings form, the way every setting of
/// QNC-AI is drawn (user 2026-11-23: "analiziraj GUI kompletne aplikacije i uskladi"): each a row
/// with its label on the left, its field, the button column empty. A choice is a list, a number is
/// typed (or picked when it has choices), a length is typed as `m:ss`. Gives whether a value changed.
pub fn rows(ui: &mut egui::Ui, width: f32, settings: &mut serde_json::Value, fields: &[qnc_settings_form::FieldSpec], options: &qnc_settings_form::OptionSets, metrics: &PanelMetrics, theme: &Theme) -> bool {
    use qnc_settings_form::FieldKind;
    let mut changed = false;
    for field in fields {
        let choices = field.options.as_ref().and_then(|name| options.get(name)).map(Vec::as_slice).unwrap_or(&[]);
        let current = value_text(settings, &field.path).unwrap_or_else(|| field.default.clone());
        qnc_settings_rows::inline_row(
            ui,
            width,
            &field.label,
            metrics,
            theme,
            |ui, field_width| {
                let listed = field.kind == FieldKind::Choice || (matches!(field.kind, FieldKind::Number | FieldKind::Integer) && !choices.is_empty());
                if listed {
                    let mut value = current.clone();
                    let shown = choices.iter().find(|(option, _)| *option == current).map_or_else(|| current.clone(), |(_, label)| label.clone());
                    egui::ComboBox::from_id_salt(&field.id).width(field_width.max(40.0)).selected_text(shown).show_ui(ui, |ui| {
                        for (option, label) in choices {
                            ui.selectable_value(&mut value, option.clone(), label);
                        }
                    });
                    if value != current {
                        changed |= match field.kind {
                            FieldKind::Number | FieldKind::Integer => qnc_settings_form::set_number(settings, &field.path, &value, field.kind == FieldKind::Integer),
                            _ => set_text(settings, &field.path, value),
                        };
                    }
                    return;
                }
                // What is typed stays as typed while it is typed (a number half written, `1:`).
                let id = egui::Id::new((&field.id, "qnc-settings-block-typed"));
                let stored = match field.kind {
                    FieldKind::Duration => current.parse::<i64>().map(qnc_settings_form::minutes_seconds).unwrap_or_default(),
                    _ => current.clone(),
                };
                let mut typed = ui.data(|data| data.get_temp::<(String, String)>(id)).filter(|(was, _)| *was == stored).map_or_else(|| stored.clone(), |(_, typed)| typed);
                let response = ui.add(qnc_ui_kit::text_field(&mut typed).desired_width(field_width.max(40.0)).hint_text(&field.hint));
                if response.changed() {
                    changed |= match field.kind {
                        FieldKind::Number | FieldKind::Integer => qnc_settings_form::set_number(settings, &field.path, &typed, field.kind == FieldKind::Integer),
                        FieldKind::Duration => qnc_settings_form::parse_seconds(&typed).is_some_and(|seconds| set_value(settings, &field.path, serde_json::Value::from(seconds))),
                        _ => set_text(settings, &field.path, typed.clone()),
                    };
                }
                let now = value_text(settings, &field.path).unwrap_or_else(|| field.default.clone());
                let now = match field.kind {
                    FieldKind::Duration => now.parse::<i64>().map(qnc_settings_form::minutes_seconds).unwrap_or_default(),
                    _ => now,
                };
                ui.data_mut(|data| data.insert_temp(id, (now, typed)));
            },
            |_, _| {},
        );
    }
    changed
}

fn value_text(settings: &serde_json::Value, path: &str) -> Option<String> {
    match path.split('.').try_fold(settings, |value, key| value.get(key))? {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(on) => Some(on.to_string()),
        _ => None,
    }
}

fn set_text(settings: &mut serde_json::Value, path: &str, text: String) -> bool {
    set_value(settings, path, serde_json::Value::from(text))
}

fn set_value(settings: &mut serde_json::Value, path: &str, value: serde_json::Value) -> bool {
    let mut at = settings;
    let keys: Vec<&str> = path.split('.').collect();
    for key in &keys[..keys.len() - 1] {
        if !at.is_object() {
            *at = serde_json::Value::Object(serde_json::Map::new());
        }
        at = at.as_object_mut().map(|map| map.entry(key.to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()))).expect("object");
    }
    if !at.is_object() {
        *at = serde_json::Value::Object(serde_json::Map::new());
    }
    at.as_object_mut().map(|map| map.insert(keys[keys.len() - 1].to_string(), value));
    true
}
