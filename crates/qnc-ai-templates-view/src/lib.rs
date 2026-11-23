//! A group of QNC-AI's AI settings (user 2026-11-23: "formu postavke podijeli na kocke ovisno o
//! grupama"): the AI templates of this computer (`qnc-ai-features`: which AI features a project
//! has, what each needs). In the window's menu the own templates and the prepared ones to start
//! from; on the right the one chosen: its name and its features, Save as an own one, Delete an own
//! one. It keeps its own draft and draws with the settings style block (`qnc_settings_block::Pen`).

use std::path::Path;

use eframe::egui;
use qnc_ai_features::{AiTemplate, Features};
use qnc_settings_block::Pen;
use qnc_wizard_text::text;

/// A template being edited: the one it came from, its name and features as drafted.
#[derive(Clone)]
struct Editing {
    from: AiTemplate,
    name: String,
    features: Vec<String>,
}

#[derive(Clone, Default)]
struct Draft {
    features: Option<Features>,
    templates: Vec<AiTemplate>,
    editing: Option<Editing>,
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
        draft
    }
}

fn id(root: &Path) -> egui::Id {
    egui::Id::new(("qnc-ai-templates-view", root.to_string_lossy().to_string()))
}

fn with<R>(ui: &mut egui::Ui, root: &Path, add: impl FnOnce(&mut egui::Ui, &mut Draft) -> R) -> R {
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id(root))).unwrap_or_else(|| Draft::read(root));
    let answer = add(ui, &mut draft);
    ui.ctx().data_mut(|data| data.insert_temp(id(root), draft));
    answer
}

/// Drops what was typed, so the next draw reads the templates again.
pub fn forget(ctx: &egui::Context, root: &Path) {
    ctx.data_mut(|data| data.remove::<Draft>(id(root)));
}

/// The templates in the window's menu, the one edited marked while `shown`; gives whether one was
/// picked (a prepared one starts a new own one: its name is typed).
pub fn menu(ui: &mut egui::Ui, root: &Path, pen: &Pen, shown: bool) -> bool {
    with(ui, root, |ui, draft| {
        let edited = draft.editing.as_ref().filter(|_| shown).map(|editing| editing.from.id.clone());
        let mut picked = None;
        pen.menu_heading(ui, &text("ai_settings.own"));
        if !draft.templates.iter().any(|template| template.own) {
            pen.hint(ui, &text("ai_settings.none_own"));
        }
        for own in [true, false] {
            if !own {
                pen.menu_heading(ui, &text("ai_settings.prepared"));
            }
            let hover = if own { String::new() } else { text("ai_settings.new_from_hint") };
            for template in draft.templates.iter().filter(|template| template.own == own) {
                if pen.menu_item(ui, edited.as_deref() == Some(template.id.as_str()), &qnc_ai_features::Choice::template_name(template), &hover) {
                    picked = Some(template.clone());
                }
            }
        }
        let Some(template) = picked else { return false };
        let name = if template.own { template.name.clone() } else { String::new() };
        draft.editing = Some(Editing { features: template.features.clone(), from: template, name });
        draft.said = None;
        true
    })
}

/// The template picked: its name and its features, each on or off, what it needs turned on with
/// it and one needed by another not turned off; Save keeps it as an own one, Delete deletes an own one.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    with(ui, root, |ui, draft| {
        if let (Some(features), Some(editing)) = (draft.features.clone(), draft.editing.as_mut()) {
            let mut refused = None;
            pen.text(ui, "export_template.name", &mut editing.name, &text("export_template.name_hint"));
            if !editing.from.own {
                pen.hint(ui, &text("export_template.from").replace("{preset}", &qnc_ai_features::Choice::template_name(&editing.from)));
            }
            pen.section(ui, &text("ai_settings.features"), |ui| {
                for feature in &features.list {
                    let mut on = editing.features.contains(&feature.code);
                    let needs: Vec<String> = features.all_needs(&feature.code).iter().filter_map(|code| features.get(code)).map(|need| qnc_ai_features::words_of(&need.name)).collect();
                    let needs = if needs.is_empty() { String::new() } else { text("ai_settings.needs").replace("{features}", &needs.join(", ")) };
                    if pen.check(ui, &qnc_ai_features::words_of(&feature.name), &mut on, &needs) {
                        if on {
                            features.turn_on(&mut editing.features, &feature.code);
                        } else if let Err(by) = features.turn_off(&mut editing.features, &feature.code) {
                            let names: Vec<String> = by.iter().map(|code| qnc_ai_features::words_of(code)).collect();
                            refused = Some(text("ai_settings.needed_by").replace("{features}", &names.join(", ")));
                        }
                    }
                }
            });
            if let Some(refused) = refused {
                draft.said = Some(Err(refused));
            }
            write(ui, root, pen, draft);
        }
        pen.said(ui, &draft.said);
    });
}

/// Save and Delete of the template edited.
fn write(ui: &mut egui::Ui, root: &Path, pen: &Pen, draft: &mut Draft) {
    let Some(editing) = draft.editing.clone() else { return };
    let (mut save, mut delete) = (false, false);
    pen.buttons(ui, |ui, buttons| {
        save = buttons.primary(ui, &text("machine.save"), !editing.name.trim().is_empty());
        if editing.from.own {
            delete = buttons.action(ui, &text("export_template.delete"));
        }
    });
    if save {
        match qnc_ai_features::keep_template(root, &editing.name, &editing.features, &draft.templates) {
            Ok(kept) => {
                *draft = Draft::read(root);
                draft.editing = Some(Editing { name: kept.name.clone(), features: kept.features.clone(), from: kept });
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
    if delete {
        match qnc_ai_features::delete_template(root, &editing.from) {
            Ok(()) => {
                *draft = Draft::read(root);
                draft.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => draft.said = Some(Err(error)),
        }
    }
}
