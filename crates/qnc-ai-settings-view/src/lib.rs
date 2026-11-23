//! A block of QNC-AI's Settings form with its own window (user 2026-11-23: "sljedeće neka budu AI
//! postavke, isto samostalna forma"; "dodaj sve što imamo i što je planirano"): the AI settings of
//! this computer, in the window every such block opens (`qnc-settings-window`).
//!
//! In the Settings form it is one row and a button. In the window, on the left:
//! - AI templates: the own ones and the prepared ones to start from (`qnc-ai-features`: which AI
//!   features a project has, what each needs);
//! - then, apart under a line: AI engines, AI server and LLM, the shared AI folder, shot labels.
//!
//! On the right what was chosen. Everything is drawn by the settings style block
//! (`qnc_settings_block::Pen`), nothing here has measures or widgets of its own (user 2026-11-23:
//! "gradi kocku, ne krpaj izgled"). The files are the ones the AI pieces already read
//! (`data/ai-templates`, `ai-place.json`, `llm.qncllm`, `ai-share.json`): QNC reads some of them too,
//! so their names stay.

mod engines;
mod labels;

use std::path::Path;

use eframe::egui;
use qnc_ai_features::{AiTemplate, Features};
use qnc_settings_block::Pen;
use qnc_settings_window::Pane;
use qnc_wizard_text::text;

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
    /// This computer's AI catalog, read when the engines are chosen; what is typed is its draft.
    catalog: Option<Result<qnc_ai_catalog_edit::Catalog, String>>,
    /// This computer's shot labels, read when they are chosen; what is typed is their draft.
    labels: Option<Result<qnc_shot_labels_edit::List, String>>,
    new_label: labels::New,
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
pub fn show(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let id = egui::Id::new(("qnc-ai-settings-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    pen.title(ui, "ai_settings.title", "ai_settings.hint");
    let mut open = false;
    pen.row(ui, &text("ai_settings.row"), |_, _| {}, |ui, buttons| open = buttons.primary(ui, &text("export_template.open"), true));
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
        draft.open = qnc_settings_window::show(ui.ctx(), root, LOOK, &text("ai_settings.window_title"), pen, |ui, pane, pen| match pane {
            Pane::Menu => menu(ui, root, pen, &mut draft),
            Pane::Panel => panel(ui, root, pen, &mut draft),
        });
    } else {
        pen.said(ui, &draft.said);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}

/// The menu: the AI templates (own, prepared), then apart under a line the other AI settings.
fn menu(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft) {
    let mut picked = None;
    let chosen_id = match &draft.chosen {
        Some(Chosen::Template { from, .. }) => Some(from.id.clone()),
        _ => None,
    };
    pen.menu_heading(ui, &text("ai_settings.own"));
    let own: Vec<&AiTemplate> = draft.templates.iter().filter(|template| template.own).collect();
    if own.is_empty() {
        pen.hint(ui, &text("ai_settings.none_own"));
    }
    for template in own {
        if pen.menu_item(ui, chosen_id.as_deref() == Some(template.id.as_str()), &qnc_ai_features::Choice::template_name(template), "") {
            picked = Some(template.clone());
        }
    }
    pen.menu_heading(ui, &text("ai_settings.prepared"));
    for template in draft.templates.iter().filter(|template| !template.own) {
        if pen.menu_item(ui, chosen_id.as_deref() == Some(template.id.as_str()), &qnc_ai_features::Choice::template_name(template), &text("ai_settings.new_from_hint")) {
            picked = Some(template.clone());
        }
    }
    if let Some(template) = picked {
        // A prepared one is the start of a new own one: its name is typed.
        let name = if template.own { template.name.clone() } else { String::new() };
        draft.chosen = Some(Chosen::Template { features: template.features.clone(), from: template, name });
        draft.said = None;
    }
    pen.menu_rule(ui);
    for (item, code) in [(Chosen::Engines, "ai_settings.engines"), (Chosen::Server, "ai_settings.server"), (Chosen::Share, "ai_settings.share"), (Chosen::Labels, "ai_settings.labels")] {
        if pen.menu_item(ui, draft.chosen.as_ref() == Some(&item), &text(code), "") {
            match item {
                Chosen::Share => draft.share = qnc_ai_catalog::chosen_share(root).map(|folder| folder.display().to_string()).unwrap_or_default(),
                Chosen::Engines => draft.catalog = Some(qnc_ai_catalog_edit::Catalog::installed()),
                Chosen::Labels => draft.labels = Some(qnc_shot_labels_edit::List::installed()),
                _ => {}
            }
            draft.chosen = Some(item);
            draft.said = None;
        }
    }
}

/// What was chosen on the left.
fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft) {
    match draft.chosen.clone() {
        None => pen.hint(ui, &text("ai_settings.choose")),
        Some(Chosen::Template { from, name, features }) => template(ui, root, pen, draft, from, name, features),
        Some(Chosen::Server) => pen.section(ui, &text("ai_settings.server"), |ui| qnc_ai_server_view::panel(ui, root, pen)),
        Some(Chosen::Share) => share(ui, root, pen, draft),
        Some(Chosen::Engines) => match draft.catalog.as_mut() {
            Some(Ok(catalog)) => {
                if let Some(said) = engines::show(ui, root, pen, catalog) {
                    draft.said = Some(said);
                }
            }
            Some(Err(error)) => draft.said = Some(Err(error.clone())),
            None => {}
        },
        Some(Chosen::Labels) => match draft.labels.as_mut() {
            Some(Ok(list)) => {
                if let Some(said) = labels::show(ui, pen, list, &mut draft.new_label) {
                    draft.said = Some(said);
                }
            }
            Some(Err(error)) => draft.said = Some(Err(error.clone())),
            None => {}
        },
    }
    pen.said(ui, &draft.said);
}

/// An AI template: its name and its features, each on or off, what it needs turned on with it and
/// one needed by another not turned off; Save keeps it as an own one, Delete deletes an own one.
fn template(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft, from: AiTemplate, mut name: String, mut chosen: Vec<String>) {
    let Some(features) = draft.features.clone() else { return };
    pen.text(ui, "export_template.name", &mut name, &text("export_template.name_hint"));
    if !from.own {
        pen.hint(ui, &text("export_template.from").replace("{preset}", &qnc_ai_features::Choice::template_name(&from)));
    }
    let mut refused = None;
    pen.section(ui, &text("ai_settings.features"), |ui| {
        for feature in &features.list {
            let mut on = chosen.contains(&feature.code);
            let needs: Vec<String> = features.all_needs(&feature.code).iter().filter_map(|code| features.get(code)).map(|need| qnc_ai_features::words_of(&need.name)).collect();
            let needs = if needs.is_empty() { String::new() } else { text("ai_settings.needs").replace("{features}", &needs.join(", ")) };
            if pen.check(ui, &qnc_ai_features::words_of(&feature.name), &mut on, &needs) {
                if on {
                    features.turn_on(&mut chosen, &feature.code);
                } else if let Err(by) = features.turn_off(&mut chosen, &feature.code) {
                    let names: Vec<String> = by.iter().map(|code| qnc_ai_features::words_of(code)).collect();
                    refused = Some(text("ai_settings.needed_by").replace("{features}", &names.join(", ")));
                }
            }
        }
    });
    if let Some(refused) = refused {
        draft.said = Some(Err(refused));
    }
    draft.chosen = Some(Chosen::Template { from: from.clone(), name: name.clone(), features: chosen.clone() });
    let (mut save, mut delete) = (false, false);
    pen.buttons(ui, |ui, buttons| {
        save = buttons.primary(ui, &text("machine.save"), !name.trim().is_empty());
        if from.own {
            delete = buttons.action(ui, &text("export_template.delete"));
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
fn share(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft) {
    pen.section(ui, &text("ai_settings.share"), |ui| {
        pen.text(ui, "ai_settings.share_folder", &mut draft.share, "");
        pen.hint(ui, &text("ai_settings.share_hint"));
        let mut save = false;
        let can = !draft.share.trim().is_empty();
        pen.buttons(ui, |ui, buttons| save = buttons.primary(ui, &text("machine.save"), can));
        if save {
            draft.said = Some(qnc_ai_catalog::set_share(root, Path::new(draft.share.trim())).map(|()| text("machine.saved")));
        }
    });
}
