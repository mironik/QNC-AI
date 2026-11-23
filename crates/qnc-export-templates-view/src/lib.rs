//! A block of QNC-AI's Settings form with its own window (user 2026-11-23: "samostalna kocka koja ima
//! svoj GUI koji će se pozivati iz Forme Postavke; na taj način forma postavke bit će mala"): the
//! export templates of this computer (`qnc-export-templates`).
//!
//! In the Settings form it is one row: how many templates there are and a button that opens its
//! window. In the window: the templates of this computer and the prepared presets as a start; a
//! template's name, the preset it starts from, and its fields by groups (picture, sound, delivery,
//! XML) as the contract names them; New, Save, Delete, Close. A prepared preset is never written
//! over. What is typed is a draft of the window until Save.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_export_templates::{FieldKind, Template};
use qnc_settings_block::{said, title};
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;
use serde_json::{Map, Value};

#[derive(Clone, Default)]
struct Draft {
    open: bool,
    templates: Vec<Template>,
    /// The template being edited (a new one has no file yet).
    editing: Option<Template>,
    /// The fields as typed, by their key (a number or text typed is kept as typed until Save).
    typed: Map<String, Value>,
    said: Option<Result<String, String>>,
}

impl Draft {
    fn read(root: &Path) -> Self {
        let mut draft = Self::default();
        match qnc_export_templates::templates(root) {
            Ok(templates) => draft.templates = templates,
            Err(error) => draft.said = Some(Err(error)),
        }
        draft
    }

    fn edit(&mut self, template: Template) {
        self.typed = template.values.clone();
        self.editing = Some(template);
    }

    /// A new template from the prepared preset `from`.
    fn new_from(&mut self, from: &qnc_export_templates::Prepared) {
        self.edit(Template { id: String::new(), name: String::new(), from: from.id.clone(), values: from.values.clone() });
    }
}

/// Draws the block's row in the Settings form; its window opens over the form.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-export-templates-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    title(ui, "export_template.title", "export_template.hint", metrics, theme);
    let count = text("export_template.count").replace("{count}", &draft.templates.len().to_string());
    let mut open = false;
    qnc_settings_rows::inline_row(
        ui,
        width,
        &count,
        metrics,
        theme,
        |_, _| {},
        |ui, _| open = qnc_settings_rows::primary_btn(ui, &text("export_template.open"), true, theme).clicked(),
    );
    if open {
        draft = Draft::read(root);
        draft.open = true;
    }
    if draft.open {
        window(ui.ctx(), root, metrics, theme, &mut draft);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}

fn window(ctx: &egui::Context, root: &Path, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    let mut open = true;
    egui::Window::new(text("export_template.window_title"))
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_size([760.0, 560.0])
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    list(ui, metrics, theme, draft);
                });
                ui.separator();
                ui.vertical(|ui| editor(ui, root, metrics, theme, draft));
            });
        });
    if !open {
        draft.open = false;
    }
}

/// The templates of this computer, then the prepared presets a new one starts from.
fn list(ui: &mut egui::Ui, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    let heading = |ui: &mut egui::Ui, code: &str| {
        ui.label(RichText::new(text(code).to_uppercase()).size(metrics.label_font_size).strong().color(theme.text_muted));
    };
    heading(ui, "export_template.own");
    if draft.templates.is_empty() {
        ui.label(RichText::new(text("export_template.none_own")).size(metrics.label_font_size).color(theme.text_muted));
    }
    let mut chosen = None;
    for template in &draft.templates {
        let selected = draft.editing.as_ref().is_some_and(|editing| editing.id == template.id);
        if ui.selectable_label(selected, &template.name).clicked() {
            chosen = Some(template.clone());
        }
    }
    if let Some(template) = chosen {
        draft.edit(template);
    }
    ui.add_space(12.0);
    heading(ui, "export_template.prepared");
    let mut start = None;
    for preset in qnc_export_templates::prepared() {
        if ui.selectable_label(false, &preset.name).on_hover_text(text("export_template.new_from_hint")).clicked() {
            start = Some(preset);
        }
    }
    if let Some(preset) = start {
        draft.new_from(&preset);
    }
}

/// The template being edited: its name, the preset it starts from, its fields; Save, Delete.
fn editor(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    let Some(mut editing) = draft.editing.clone() else {
        ui.label(RichText::new(text("export_template.choose")).size(metrics.label_font_size).color(theme.text_muted));
        said(ui, &draft.said, theme);
        return;
    };
    let width = ui.available_width();
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text("export_template.name"),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(&mut editing.name).desired_width(field_width.max(80.0)));
        },
        |_, _| {},
    );
    let from = qnc_export_templates::prepared().into_iter().find(|preset| preset.id == editing.from).map_or_else(|| editing.from.clone(), |preset| preset.name);
    ui.label(RichText::new(text("export_template.from").replace("{preset}", &from)).size(metrics.label_font_size).color(theme.text_muted));
    let groups = match qnc_export_templates::field_groups() {
        Ok(groups) => groups,
        Err(error) => {
            ui.colored_label(theme.danger, error);
            return;
        }
    };
    egui::ScrollArea::vertical().max_height((ui.available_height() - 60.0).max(120.0)).show(ui, |ui| {
        for group in &groups {
            ui.add_space(8.0);
            ui.label(RichText::new(text(&group.label).to_uppercase()).size(metrics.label_font_size).strong().color(theme.text_muted));
            for field in &group.fields {
                let label = text(&field.label);
                qnc_settings_rows::inline_row(ui, width, &label, metrics, theme, |ui, field_width| field_widget(ui, field, field_width, draft), |_, _| {});
            }
        }
    });
    draft.editing = Some(editing.clone());
    let (mut save, mut delete) = (false, false);
    let is_own = draft.templates.iter().any(|template| template.id == editing.id);
    ui.horizontal(|ui| {
        save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), !editing.name.trim().is_empty(), theme).clicked();
        if is_own {
            delete = qnc_settings_rows::action_btn(ui, &text("export_template.delete"), theme).clicked();
        }
    });
    if save {
        let values = draft.typed.iter().filter(|(_, value)| !matches!(value, Value::String(text) if text.trim().is_empty())).map(|(key, value)| (key.clone(), value.clone())).collect();
        let id = if is_own { editing.id.clone() } else { qnc_export_templates::id_of(&editing.name) };
        let template = Template { id, name: editing.name.trim().to_string(), from: editing.from.clone(), values };
        match qnc_export_templates::save(root, &template) {
            Ok(()) => {
                let open = draft.open;
                *draft = Draft::read(root);
                draft.open = open;
                draft.edit(template);
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    if delete {
        match qnc_export_templates::delete(root, &editing.id) {
            Ok(()) => {
                let open = draft.open;
                *draft = Draft::read(root);
                draft.open = open;
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    said(ui, &draft.said, theme);
}

/// A field's widget: a choice of the values the prepared presets have, a number or a text typed,
/// or a switch.
fn field_widget(ui: &mut egui::Ui, field: &qnc_export_templates::Field, width: f32, draft: &mut Draft) {
    let shown = |value: Option<&Value>| match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    };
    match field.kind {
        FieldKind::Choice => {
            let now = draft.typed.get(&field.key).cloned();
            let mut picked = None;
            egui::ComboBox::from_id_salt(("qnc-export-template", field.key.as_str())).width(width.max(120.0)).selected_text(shown(now.as_ref())).show_ui(ui, |ui| {
                for value in qnc_export_templates::choices(&field.key) {
                    if ui.selectable_label(now.as_ref() == Some(&value), shown(Some(&value))).clicked() {
                        picked = Some(value);
                    }
                }
            });
            if let Some(value) = picked {
                draft.typed.insert(field.key.clone(), value);
            }
        }
        FieldKind::Number => {
            let mut typed = shown(draft.typed.get(&field.key));
            if ui.add(qnc_ui_kit::text_field(&mut typed).desired_width(width.max(80.0))).changed() {
                // A number typed is kept as one when it is one, else as typed (Save keeps it so).
                let value = typed.trim().replace(',', ".");
                let number = value.parse::<i64>().map(Value::from).ok().or_else(|| value.parse::<f64>().ok().and_then(serde_json::Number::from_f64).map(Value::Number));
                draft.typed.insert(field.key.clone(), number.unwrap_or(Value::String(typed)));
            }
        }
        FieldKind::Text => {
            let mut typed = shown(draft.typed.get(&field.key));
            if ui.add(qnc_ui_kit::text_field(&mut typed).desired_width(width.max(80.0))).changed() {
                draft.typed.insert(field.key.clone(), Value::String(typed));
            }
        }
        FieldKind::Switch => {
            let mut on = draft.typed.get(&field.key).and_then(Value::as_bool).unwrap_or(false);
            if qnc_ui_kit::check_box(ui, &mut on, "", true).changed() {
                draft.typed.insert(field.key.clone(), Value::Bool(on));
            }
        }
    }
}
