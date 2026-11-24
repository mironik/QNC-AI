//! The Export tab of QNC-AI's Settings form (user 2026-11-24: "dodaj jos AI tab i Export tab i u
//! njih premjesti sadrzaj tih tabova"; before, a window of its own opened from the Settings form):
//! the export settings of this computer (`qnc-export-templates`). On the left, the tab's menu: the
//! templates of this computer and the prepared presets to start from, then, apart under a line,
//! Export XML (user 2026-11-23: "XML isto postavi u lijevi izbornik, i linijom ga odvoji od enkode
//! postavki"). On the right: what was chosen, its fields by groups as the contract names them; Save,
//! Delete. A prepared preset is never written over. What is typed is a draft of the tab until Save.
//! Everything is drawn by the settings style block (`qnc_settings_block::Pen`).

use std::path::Path;

use eframe::egui;
use qnc_export_templates::{FieldGroup, FieldKind, Template};
use qnc_settings_block::Pen;
use qnc_wizard_text::text;
use serde_json::{Map, Value};


/// What the right side shows.
#[derive(Clone, PartialEq)]
enum Chosen {
    Template(Template),
    Xml,
}

#[derive(Clone, Default)]
struct Draft {
    templates: Vec<Template>,
    chosen: Option<Chosen>,
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

    fn choose_template(&mut self, template: Template) {
        self.typed = template.values.clone();
        self.chosen = Some(Chosen::Template(template));
    }

    fn choose_xml(&mut self, root: &Path) {
        match qnc_export_templates::xml(root) {
            Ok(values) => self.typed = values,
            Err(error) => {
                self.typed = Map::new();
                self.said = Some(Err(error));
            }
        }
        self.chosen = Some(Chosen::Xml);
    }
}

/// The tab's draft, read from the templates of this computer the first time it is shown.
fn with(ui: &mut egui::Ui, root: &Path, body: impl FnOnce(&mut egui::Ui, &mut Draft)) {
    let id = egui::Id::new(("qnc-export-templates-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    body(ui, &mut draft);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}

/// The tab's menu: the templates of this computer, the prepared presets, then Export XML.
pub fn menu(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    with(ui, root, |ui, draft| list(ui, root, pen, draft));
}

/// What was chosen in the menu.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    with(ui, root, |ui, draft| editor(ui, root, pen, draft));
}

/// The templates of this computer, the prepared presets a new one starts from, then apart under a
/// line Export XML.
fn list(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft) {
    pen.menu_heading(ui, &text("export_template.own"));
    if draft.templates.is_empty() {
        pen.hint(ui, &text("export_template.none_own"));
    }
    let mut picked = None;
    for template in &draft.templates {
        let selected = matches!(&draft.chosen, Some(Chosen::Template(chosen)) if chosen.id == template.id);
        if pen.menu_item(ui, selected, &template.name, "") {
            picked = Some(template.clone());
        }
    }
    pen.menu_heading(ui, &text("export_template.prepared"));
    for preset in qnc_export_templates::prepared() {
        if pen.menu_item(ui, false, &preset.name, &text("export_template.new_from_hint")) {
            picked = Some(Template { id: String::new(), name: String::new(), from: preset.id.clone(), values: preset.values.clone() });
        }
    }
    if let Some(template) = picked {
        draft.choose_template(template);
    }
    // Export XML apart from the encode settings.
    pen.menu_rule(ui);
    if pen.menu_item(ui, draft.chosen == Some(Chosen::Xml), &text("export_template.group_xml"), "") {
        draft.choose_xml(root);
    }
}

/// What was chosen on the left: a template (its name, the preset it starts from, its fields; Save,
/// Delete) or Export XML (its fields; Save).
fn editor(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft) {
    match draft.chosen.clone() {
        None => pen.hint(ui, &text("export_template.choose")),
        Some(Chosen::Xml) => match qnc_export_templates::xml_fields() {
            Ok(group) => {
                fields(ui, pen, &[group], draft);
                let mut save = false;
                pen.buttons(ui, |ui, buttons| save = buttons.primary(ui, &text("machine.save"), true));
                if save {
                    draft.said = Some(qnc_export_templates::save_xml(root, &draft.typed).map(|()| text("machine.saved")));
                }
            }
            Err(error) => pen.said(ui, &Some(Err(error))),
        },
        Some(Chosen::Template(template)) => template_editor(ui, root, pen, draft, template),
    }
    pen.said(ui, &draft.said);
}

fn template_editor(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft, mut editing: Template) {
    pen.text(ui, "export_template.name", &mut editing.name, &text("export_template.name_hint"));
    let from = qnc_export_templates::prepared().into_iter().find(|preset| preset.id == editing.from).map_or_else(|| editing.from.clone(), |preset| preset.name);
    pen.hint(ui, &text("export_template.from").replace("{preset}", &from));
    let groups = match qnc_export_templates::field_groups() {
        Ok(groups) => groups,
        Err(error) => {
            pen.said(ui, &Some(Err(error)));
            return;
        }
    };
    let rows = (ui.available_height() / pen.rows_height(ui, 1.0) - 2.0).floor().max(3.0);
    pen.list(ui, "qnc-export-template-fields", rows, |ui, pen| fields(ui, pen, &groups, draft));
    draft.chosen = Some(Chosen::Template(editing.clone()));
    let is_own = draft.templates.iter().any(|template| template.id == editing.id);
    let (mut save, mut delete) = (false, false);
    pen.listed().buttons(ui, |ui, buttons| {
        save = buttons.primary(ui, &text("machine.save"), !editing.name.trim().is_empty());
        if is_own {
            delete = buttons.action(ui, &text("export_template.delete"));
        }
    });
    if save {
        let values = draft.typed.iter().filter(|(_, value)| !matches!(value, Value::String(text) if text.trim().is_empty())).map(|(key, value)| (key.clone(), value.clone())).collect();
        let id = if is_own { editing.id.clone() } else { qnc_export_templates::id_of(&editing.name) };
        let template = Template { id, name: editing.name.trim().to_string(), from: editing.from.clone(), values };
        match qnc_export_templates::save(root, &template) {
            Ok(()) => {
                draft.templates = qnc_export_templates::templates(root).unwrap_or_default();
                draft.choose_template(template);
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    if delete {
        match qnc_export_templates::delete(root, &editing.id) {
            Ok(()) => {
                draft.templates = qnc_export_templates::templates(root).unwrap_or_default();
                draft.chosen = None;
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
}

/// The fields of `groups`, each group a section, each field a row; a switch a row with its box.
fn fields(ui: &mut egui::Ui, pen: &Pen, groups: &[FieldGroup], draft: &mut Draft) {
    let mut settings = serde_json::json!({ "export": Value::Object(draft.typed.clone()) });
    for group in groups {
        pen.section(ui, &text(&group.label), |ui| {
            let mut options = qnc_settings_form::OptionSets::new();
            let specs: Vec<qnc_settings_form::FieldSpec> = group
                .fields
                .iter()
                .filter_map(|field| {
                    let kind = match field.kind {
                        FieldKind::Choice => qnc_settings_form::FieldKind::Choice,
                        FieldKind::Number => qnc_settings_form::FieldKind::Number,
                        FieldKind::Integer => qnc_settings_form::FieldKind::Integer,
                        FieldKind::Text => qnc_settings_form::FieldKind::Text,
                        FieldKind::Switch => return None,
                    };
                    if field.kind == FieldKind::Choice {
                        let values = qnc_export_templates::choices(&field.key).into_iter().map(|value| value.as_str().map_or_else(|| value.to_string(), str::to_string)).map(|value| (value.clone(), value)).collect();
                        options.insert(field.key.clone(), values);
                    }
                    Some(qnc_settings_form::FieldSpec {
                        kind,
                        label: text(&field.label),
                        id: format!("qnc-export-template-{}", field.key),
                        path: format!("export.{}", field.key),
                        options: (field.kind == FieldKind::Choice).then(|| field.key.clone()),
                        default: String::new(),
                        hint: String::new(),
                    })
                })
                .collect();
            pen.fields(ui, &mut settings, &specs, &options);
            for field in group.fields.iter().filter(|field| field.kind == FieldKind::Switch) {
                let mut on = settings["export"].get(&field.key).and_then(Value::as_bool).unwrap_or(false);
                if pen.check(ui, &text(&field.label), &mut on, "") {
                    settings["export"][&field.key] = Value::Bool(on);
                }
            }
        });
    }
    if let Some(typed) = settings.get("export").and_then(Value::as_object) {
        draft.typed = typed.clone();
    }
}
