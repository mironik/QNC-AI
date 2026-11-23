//! A block of QNC-AI's Settings form with its own window (user 2026-11-23: "sljedeće neka budu AI
//! postavke, isto samostalna forma"; "dodaj sve što imamo i što je planirano"): the AI settings of
//! this computer, in the window every such block opens (`qnc-settings-window`).
//!
//! In the Settings form it is one row and a button. In the window, on the left:
//! - AI templates: the own ones and the prepared ones to start from (`qnc-ai-features`: which AI
//!   features a project has, what each needs);
//! - then, apart under a line: AI engines, AI server and LLM, the shared AI folder, shot labels.
//!
//! On the right what was chosen, its fields in the Settings form's grid. The files are the ones the
//! AI pieces already read (`data/ai-templates`, `ai-place.json`, `llm.qncllm`, `ai-share.json`):
//! QNC reads some of them too, so their names stay.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_ai_features::{AiTemplate, Features};
use qnc_desktop_theme::Theme;
use qnc_settings_block::{said, title};
use qnc_settings_rows::PanelMetrics;
use qnc_settings_window::{Pane, Style};
use qnc_wizard_text::text;
use serde_json::Value;

/// The form name of the window's look.
const LOOK: &str = "ai-settings";

/// What the right side shows.
#[derive(Clone, PartialEq)]
enum Chosen {
    /// A template being edited: the one it came from, its name and features as drafted.
    Template { from: AiTemplate, name: String, features: Vec<String> },
    Engines,
    Server,
    Share,
    Labels,
}

#[derive(Clone, Default)]
struct Draft {
    open: bool,
    features: Option<Features>,
    templates: Vec<AiTemplate>,
    chosen: Option<Chosen>,
    /// The shared AI folder as typed.
    share: String,
    said: Option<Result<String, String>>,
}

impl Draft {
    fn read(root: &Path) -> Self {
        let mut draft = Self::default();
        match Features::load(root) {
            Ok(features) => {
                let (templates, errors) = qnc_ai_features::templates(root, &features);
                draft.templates = templates;
                if !errors.is_empty() {
                    draft.said = Some(Err(errors.join("; ")));
                }
                draft.features = Some(features);
            }
            Err(error) => draft.said = Some(Err(error)),
        }
        draft.share = qnc_ai_catalog::chosen_share(root).map(|folder| folder.display().to_string()).unwrap_or_default();
        draft
    }
}

/// Draws the block's row in the Settings form; its window opens in the middle of the screen.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-ai-settings-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    title(ui, "ai_settings.title", "ai_settings.hint", metrics, theme);
    let mut open = false;
    qnc_settings_rows::inline_row(ui, width, &text("ai_settings.row"), metrics, theme, |_, _| {}, |ui, _| open = qnc_settings_rows::primary_btn(ui, &text("export_template.open"), true, theme).clicked());
    if open && !draft.open {
        match qnc_settings_window::open(root, LOOK, &text("ai_settings.window_title")) {
            Ok(()) => {
                draft = Draft::read(root);
                draft.open = true;
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    if draft.open {
        draft.open = qnc_settings_window::show(ui.ctx(), root, LOOK, &text("ai_settings.window_title"), metrics, theme, |ui, pane, style| match pane {
            Pane::Menu => menu(ui, root, style, theme, &mut draft),
            Pane::Panel => panel(ui, root, style, theme, &mut draft),
        });
    } else {
        said(ui, &draft.said, theme);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}

/// The menu: the AI templates (own, prepared), then apart under a line the other AI settings.
fn menu(ui: &mut egui::Ui, root: &Path, style: &Style, theme: &Theme, draft: &mut Draft) {
    let size = style.metrics.label_font_size;
    let heading = |ui: &mut egui::Ui, code: &str| {
        ui.label(RichText::new(text(code).to_uppercase()).size(size).strong().color(theme.text_muted));
    };
    let mut picked = None;
    let chosen_id = match &draft.chosen {
        Some(Chosen::Template { from, .. }) => Some(from.id.clone()),
        _ => None,
    };
    heading(ui, "ai_settings.own");
    let own: Vec<&AiTemplate> = draft.templates.iter().filter(|template| template.own).collect();
    if own.is_empty() {
        ui.label(RichText::new(text("ai_settings.none_own")).size(size).color(theme.text_muted));
    }
    for template in own {
        if ui.selectable_label(chosen_id.as_deref() == Some(template.id.as_str()), qnc_ai_features::Choice::template_name(template)).clicked() {
            picked = Some(template.clone());
        }
    }
    ui.add_space(12.0);
    heading(ui, "ai_settings.prepared");
    for template in draft.templates.iter().filter(|template| !template.own) {
        if ui.selectable_label(chosen_id.as_deref() == Some(template.id.as_str()), qnc_ai_features::Choice::template_name(template)).on_hover_text(text("ai_settings.new_from_hint")).clicked() {
            picked = Some(template.clone());
        }
    }
    if let Some(template) = picked {
        // A prepared one is the start of a new own one: its name is typed.
        let name = if template.own { template.name.clone() } else { String::new() };
        draft.chosen = Some(Chosen::Template { features: template.features.clone(), from: template, name });
        draft.said = None;
    }
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(4.0);
    for (item, code) in [(Chosen::Engines, "ai_settings.engines"), (Chosen::Server, "ai_settings.server"), (Chosen::Share, "ai_settings.share"), (Chosen::Labels, "ai_settings.labels")] {
        if ui.selectable_label(draft.chosen.as_ref() == Some(&item), text(code)).clicked() {
            if item == Chosen::Share {
                draft.share = qnc_ai_catalog::chosen_share(root).map(|folder| folder.display().to_string()).unwrap_or_default();
            }
            draft.chosen = Some(item);
            draft.said = None;
        }
    }
}

/// What was chosen on the left.
fn panel(ui: &mut egui::Ui, root: &Path, style: &Style, theme: &Theme, draft: &mut Draft) {
    let width = ui.available_width();
    match draft.chosen.clone() {
        None => {
            ui.label(RichText::new(text("ai_settings.choose")).size(style.metrics.label_font_size).color(theme.text_muted));
        }
        Some(Chosen::Template { from, name, features }) => template(ui, root, width, style, theme, draft, from, name, features),
        Some(Chosen::Server) => {
            qnc_settings_rows::section(ui, width, &text("ai_settings.server"), &style.metrics, theme, |ui| qnc_ai_server_view::panel(ui, root, width, &style.grid, theme));
        }
        Some(Chosen::Share) => share(ui, root, width, style, theme, draft),
        Some(Chosen::Engines) | Some(Chosen::Labels) => {
            ui.label(RichText::new(text("ai_settings.next_step")).size(style.metrics.label_font_size).color(theme.text_muted));
        }
    }
    said(ui, &draft.said, theme);
}

/// An AI template: its name and its features, each on or off, what it needs turned on with it and
/// one needed by another not turned off; Save keeps it as an own one, Delete deletes an own one.
#[allow(clippy::too_many_arguments)]
fn template(ui: &mut egui::Ui, root: &Path, width: f32, style: &Style, theme: &Theme, draft: &mut Draft, from: AiTemplate, mut name: String, mut chosen: Vec<String>) {
    let Some(features) = draft.features.clone() else { return };
    let mut named = serde_json::json!({ "name": name });
    let spec = qnc_settings_form::FieldSpec {
        kind: qnc_settings_form::FieldKind::Text,
        label: text("export_template.name"),
        id: "qnc-ai-template-name".into(),
        path: "name".into(),
        options: None,
        default: String::new(),
        hint: text("export_template.name_hint"),
    };
    qnc_settings_form::field_grid(ui, width, &mut named, std::slice::from_ref(&spec), &qnc_settings_form::OptionSets::new(), &style.grid);
    name = named.get("name").and_then(Value::as_str).unwrap_or_default().to_string();
    if !from.own {
        let start = text("export_template.from").replace("{preset}", &qnc_ai_features::Choice::template_name(&from));
        ui.label(RichText::new(start).size(style.grid.label_size).color(theme.text_muted));
    }
    let mut refused = None;
    qnc_settings_rows::section(ui, width, &text("ai_settings.features"), &style.metrics, theme, |ui| {
        for feature in &features.list {
            let mut on = chosen.contains(&feature.code);
            let label = RichText::new(qnc_ai_features::words_of(&feature.name)).size(style.grid.label_size).color(theme.text);
            if qnc_ui_kit::check_box(ui, &mut on, label, true).changed() {
                if on {
                    features.turn_on(&mut chosen, &feature.code);
                } else if let Err(by) = features.turn_off(&mut chosen, &feature.code) {
                    let names: Vec<String> = by.iter().map(|code| qnc_ai_features::words_of(code)).collect();
                    refused = Some(text("ai_settings.needed_by").replace("{features}", &names.join(", ")));
                }
            }
            let needs: Vec<String> = features.all_needs(&feature.code).iter().filter_map(|code| features.get(code)).map(|need| qnc_ai_features::words_of(&need.name)).collect();
            if !needs.is_empty() {
                ui.label(RichText::new(text("ai_settings.needs").replace("{features}", &needs.join(", "))).size(style.grid.label_size).color(theme.text_muted));
            }
        }
    });
    if let Some(refused) = refused {
        draft.said = Some(Err(refused));
    }
    draft.chosen = Some(Chosen::Template { from: from.clone(), name: name.clone(), features: chosen.clone() });
    let (mut save, mut delete) = (false, false);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), !name.trim().is_empty(), theme).clicked();
        if from.own {
            delete = qnc_settings_rows::action_btn(ui, &text("export_template.delete"), theme).clicked();
        }
    });
    if save {
        match qnc_ai_features::keep_template(root, &name, &chosen, &draft.templates) {
            Ok(kept) => {
                let open = draft.open;
                *draft = Draft::read(root);
                draft.open = open;
                draft.chosen = Some(Chosen::Template { name: kept.name.clone(), features: kept.features.clone(), from: kept });
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    if delete {
        match qnc_ai_features::delete_template(root, &from) {
            Ok(()) => {
                let open = draft.open;
                *draft = Draft::read(root);
                draft.open = open;
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
}

/// The shared AI folder of this computer (where the AI models are, for QNC and QNC-AI).
fn share(ui: &mut egui::Ui, root: &Path, width: f32, style: &Style, theme: &Theme, draft: &mut Draft) {
    qnc_settings_rows::section(ui, width, &text("ai_settings.share"), &style.metrics, theme, |ui| {
        let mut settings = serde_json::json!({ "folder": draft.share });
        let spec = qnc_settings_form::FieldSpec {
            kind: qnc_settings_form::FieldKind::Text,
            label: text("ai_settings.share_folder"),
            id: "qnc-ai-share-folder".into(),
            path: "folder".into(),
            options: None,
            default: String::new(),
            hint: String::new(),
        };
        qnc_settings_form::field_grid(ui, width, &mut settings, std::slice::from_ref(&spec), &qnc_settings_form::OptionSets::new(), &style.grid);
        draft.share = settings.get("folder").and_then(Value::as_str).unwrap_or_default().to_string();
        ui.label(RichText::new(text("ai_settings.share_hint")).size(style.grid.label_size).color(theme.text_muted));
        if qnc_settings_rows::primary_btn(ui, &text("machine.save"), !draft.share.trim().is_empty(), theme).clicked() {
            draft.said = Some(qnc_ai_catalog::set_share(root, Path::new(draft.share.trim())).map(|()| text("machine.saved")));
        }
    });
}
