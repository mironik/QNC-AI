//! A block of QNC-AI's Settings form with its own window (user 2026-11-23: "samostalna kocka koja ima
//! svoj GUI koji će se pozivati iz Forme Postavke; na taj način forma postavke bit će mala"): the
//! export settings of this computer (`qnc-export-templates`).
//!
//! In the Settings form it is one row: how many templates there are and a button that opens its
//! window. The window is a window of its own in the middle of the screen (user 2026-11-23: "postavi
//! je na sredinu ekrana"), with its own bar whose hidden dot opens its own look editor (its fixed
//! size and its measures: `contracts/ui/forms/export-templates.look.json`). On the left: the
//! templates of this computer and the prepared presets to start from, then, apart under a line,
//! Export XML (user 2026-11-23: "XML isto postavi u lijevi izbornik, i linijom ga odvoji od enkode
//! postavki"). On the right: what was chosen, its fields by groups as the contract names them; Save,
//! Delete. A prepared preset is never written over. What is typed is a draft of the window until
//! Save.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_export_templates::{FieldGroup, FieldKind, Template};
use qnc_settings_block::{said, title};
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;
use serde_json::{Map, Value};

/// The form name of the window's look.
const LOOK: &str = "export-templates";

/// What the right side shows.
#[derive(Clone, PartialEq)]
enum Chosen {
    Template(Template),
    Xml,
}

#[derive(Clone, Default)]
struct Draft {
    open: bool,
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

/// Draws the block's row in the Settings form; its window opens in the middle of the screen.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-export-templates-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    title(ui, "export_template.title", "export_template.hint", metrics, theme);
    let count = text("export_template.count").replace("{count}", &draft.templates.len().to_string());
    let mut open = false;
    qnc_settings_rows::inline_row(ui, width, &count, metrics, theme, |_, _| {}, |ui, _| open = qnc_settings_rows::primary_btn(ui, &text("export_template.open"), true, theme).clicked());
    if open && !draft.open {
        match qnc_settings_window::open(root, LOOK, &text("export_template.window_title")) {
            Ok(()) => {
                draft = Draft::read(root);
                draft.open = true;
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    if draft.open {
        window(ui.ctx(), root, metrics, theme, &mut draft);
    } else {
        said(ui, &draft.said, theme);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}

/// The window of its own (`qnc-settings-window`): the menu on the left, the chosen on the right.
fn window(ctx: &egui::Context, root: &Path, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    draft.open = qnc_settings_window::show(ctx, root, LOOK, &text("export_template.window_title"), metrics, theme, |ui, pane, style| match pane {
        qnc_settings_window::Pane::Menu => list(ui, root, &style.metrics, theme, draft),
        qnc_settings_window::Pane::Panel => editor(ui, root, &style.metrics, theme, &style.grid, draft),
    });
}

/// The templates of this computer, the prepared presets a new one starts from, then apart under a
/// line Export XML.
fn list(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    let heading = |ui: &mut egui::Ui, code: &str| {
        ui.label(RichText::new(text(code).to_uppercase()).size(metrics.label_font_size).strong().color(theme.text_muted));
    };
    heading(ui, "export_template.own");
    if draft.templates.is_empty() {
        ui.label(RichText::new(text("export_template.none_own")).size(metrics.label_font_size).color(theme.text_muted));
    }
    let mut picked = None;
    for template in &draft.templates {
        let selected = matches!(&draft.chosen, Some(Chosen::Template(chosen)) if chosen.id == template.id);
        if ui.selectable_label(selected, &template.name).clicked() {
            picked = Some(template.clone());
        }
    }
    ui.add_space(12.0);
    heading(ui, "export_template.prepared");
    for preset in qnc_export_templates::prepared() {
        if ui.selectable_label(false, &preset.name).on_hover_text(text("export_template.new_from_hint")).clicked() {
            picked = Some(Template { id: String::new(), name: String::new(), from: preset.id.clone(), values: preset.values.clone() });
        }
    }
    if let Some(template) = picked {
        draft.choose_template(template);
    }
    // Export XML apart from the encode settings.
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(4.0);
    if ui.selectable_label(draft.chosen == Some(Chosen::Xml), text("export_template.group_xml")).clicked() {
        draft.choose_xml(root);
    }
}

/// What was chosen on the left: a template (its name, the preset it starts from, its fields; Save,
/// Delete) or Export XML (its fields; Save).
fn editor(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, style: &qnc_settings_form::GridStyle, draft: &mut Draft) {
    match draft.chosen.clone() {
        None => {
            ui.label(RichText::new(text("export_template.choose")).size(metrics.label_font_size).color(theme.text_muted));
        }
        Some(Chosen::Xml) => match qnc_export_templates::xml_fields() {
            Ok(group) => {
                let width = ui.available_width();
                fields(ui, &[group], width, metrics, theme, style, draft);
                let mut save = false;
                qnc_settings_rows::trailing_actions(ui, width, metrics, |ui| save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked());
                if save {
                    draft.said = Some(qnc_export_templates::save_xml(root, &draft.typed).map(|()| text("machine.saved")));
                }
            }
            Err(error) => {
                ui.colored_label(theme.danger, error);
            }
        },
        Some(Chosen::Template(template)) => template_editor(ui, root, metrics, theme, style, draft, template),
    }
    said(ui, &draft.said, theme);
}

fn template_editor(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, style: &qnc_settings_form::GridStyle, draft: &mut Draft, mut editing: Template) {
    let width = ui.available_width();
    // The name in the same grid as the fields.
    let mut named = serde_json::json!({ "name": editing.name });
    let name = qnc_settings_form::FieldSpec {
        kind: qnc_settings_form::FieldKind::Text,
        label: text("export_template.name"),
        id: "qnc-export-template-name".into(),
        path: "name".into(),
        options: None,
        default: String::new(),
        hint: text("export_template.name_hint"),
    };
    qnc_settings_block::rows(ui, width, &mut named, std::slice::from_ref(&name), &qnc_settings_form::OptionSets::new(), metrics, theme);
    editing.name = named.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    let from = qnc_export_templates::prepared().into_iter().find(|preset| preset.id == editing.from).map_or_else(|| editing.from.clone(), |preset| preset.name);
    ui.label(RichText::new(text("export_template.from").replace("{preset}", &from)).size(metrics.label_font_size).color(theme.text_muted));
    let groups = match qnc_export_templates::field_groups() {
        Ok(groups) => groups,
        Err(error) => {
            ui.colored_label(theme.danger, error);
            return;
        }
    };
    egui::ScrollArea::vertical().max_height((ui.available_height() - 60.0).max(120.0)).show(ui, |ui| fields(ui, &groups, width, metrics, theme, style, draft));
    draft.chosen = Some(Chosen::Template(editing.clone()));
    let is_own = draft.templates.iter().any(|template| template.id == editing.id);
    let (mut save, mut delete) = (false, false);
    qnc_settings_rows::trailing_actions(ui, width, metrics, |ui| {
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

/// The fields of `groups`, each group a section of the Settings form, its fields in the same grid
/// as the Advanced settings (`qnc-settings-form`: the label above its field; user 2026-11-23:
/// "treba biti stilski isto"); a switch under them.
fn fields(ui: &mut egui::Ui, groups: &[FieldGroup], width: f32, metrics: &PanelMetrics, theme: &Theme, _style: &qnc_settings_form::GridStyle, draft: &mut Draft) {
    let mut settings = serde_json::json!({ "export": Value::Object(draft.typed.clone()) });
    for group in groups {
        qnc_settings_rows::section(ui, width, &text(&group.label), metrics, theme, |ui| {
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
            qnc_settings_block::rows(ui, width, &mut settings, &specs, &options, metrics, theme);
            for field in group.fields.iter().filter(|field| field.kind == FieldKind::Switch) {
                // A switch is a row too: its label on the left, its box where the field is.
                let mut on = settings["export"].get(&field.key).and_then(Value::as_bool).unwrap_or(false);
                let mut flipped = false;
                qnc_settings_rows::inline_row(ui, width, &text(&field.label), metrics, theme, |ui, _| flipped = qnc_ui_kit::check_box(ui, &mut on, RichText::new(""), true).changed(), |_, _| {});
                if flipped {
                    settings["export"][&field.key] = Value::Bool(on);
                }
            }
        });
    }
    if let Some(typed) = settings.get("export").and_then(Value::as_object) {
        draft.typed = typed.clone();
    }
}
