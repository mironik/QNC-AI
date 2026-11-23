//! A group of the Project and Settings forms (user 2026-11-24: "formu postavke podijeli na kocke
//! ovisno o grupama"): the project templates: the picker (the templates of this computer, the one
//! chosen, Delete of an own one), New project and the open project's story sheet, the template
//! actions (New template, Cancel) and the own template being made (its name, Save, its
//! description). The picker keeps its look (a list one opens); the rest is drawn with the settings
//! style block (`qnc_settings_block::Pen`). It gives what the user asked as the Project session's
//! intents; it writes nothing itself.

use eframe::egui::{self, RichText, Sense};
use qnc_desktop_theme::Theme;
use qnc_project_session::{ProjectIntent, ProjectSession};
use qnc_project_texts::ProjectTexts;
use qnc_settings_block::Pen;

/// The picker: the title, the chosen template (a click opens the list), and while open every
/// template (a click chooses it; an own one has Delete).
pub fn picker(ui: &mut egui::Ui, pen: &Pen, theme: &Theme, texts: &ProjectTexts, session: &mut ProjectSession) -> Vec<ProjectIntent> {
    let mut intents = Vec::new();
    let font = theme.font_ui;
    let selected_name = session.templates.iter().find(|template| template.selected).map_or_else(|| "—".to_string(), |template| template.name.clone());
    ui.set_max_width(pen.width);
    pen.heading(ui, qnc_ui_text::text("project.workflow_title"));
    ui.add_space(6.0);
    let head = egui::Frame::NONE.fill(theme.surface_alt).stroke(egui::Stroke::new(1.0, theme.border)).inner_margin(egui::Margin::symmetric(8, 6)).show(ui, |ui| {
        ui.set_max_width(pen.width);
        ui.horizontal(|ui| {
            ui.label(RichText::new(selected_name).size(font).strong().color(theme.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(if session.picker_open { "▲" } else { "▼" }).size(11.0).color(theme.text_muted));
            });
        });
    });
    if head.response.interact(Sense::click()).clicked() {
        session.picker_open = !session.picker_open;
    }
    if !session.picker_open {
        return intents;
    }
    ui.add_space(pen.metrics.section_gap);
    if session.templates.is_empty() {
        pen.hint(ui, &texts.templates_empty);
        return intents;
    }
    for template in session.templates.clone() {
        let mut delete_requested = false;
        let row = egui::Frame::NONE.fill(if template.selected { theme.surface } else { theme.bg }).inner_margin(egui::Margin::symmetric(8, 5)).show(ui, |ui| {
            ui.set_max_width(pen.width);
            ui.horizontal(|ui| {
                let mut label = RichText::new(&template.name).size(font).color(theme.text);
                if template.selected {
                    label = label.strong();
                }
                ui.label(label);
                if !template.system {
                    ui.label(RichText::new("custom").size(11.0).color(theme.text_muted));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let delete = qnc_ui_kit::framed_button(RichText::new("X").size(font).color(theme.text_muted), egui::vec2(32.0, 24.0), egui::Color32::TRANSPARENT, None, false);
                    if !template.system && ui.add(delete).on_hover_text(&texts.delete_template_hint).clicked() {
                        delete_requested = true;
                    }
                });
            });
            if !template.description.trim().is_empty() {
                ui.label(RichText::new(&template.description).size(11.0).color(theme.text_muted));
            }
        });
        if delete_requested {
            intents.push(ProjectIntent::RequestDeleteTemplate { template_id: template.template_id });
        } else if row.response.interact(Sense::click()).clicked() {
            intents.push(ProjectIntent::SelectTemplate { template_id: template.template_id });
        }
    }
    intents
}

/// After the template: New project opens the story sheet, where its name is typed; the open
/// project's sheet opens with Story sheet.
pub fn project_create(ui: &mut egui::Ui, pen: &Pen, texts: &ProjectTexts, session: &ProjectSession) -> Option<ProjectIntent> {
    let can_create = !session.selected_template_id.trim().is_empty();
    let open = session.story_project.is_some();
    let mut intent = None;
    pen.buttons(ui, |ui, buttons| {
        if open && buttons.action(ui, &texts.story_sheet) {
            intent = Some(ProjectIntent::OpenStorySheet);
        }
        if buttons.primary(ui, &texts.new_project, can_create) {
            intent = Some(ProjectIntent::CreateProject);
        }
    });
    intent
}

/// New template, and Cancel while one is being made.
pub fn actions(ui: &mut egui::Ui, pen: &Pen, texts: &ProjectTexts, session: &ProjectSession) -> Option<ProjectIntent> {
    let mut intent = None;
    pen.buttons(ui, |ui, buttons| {
        if session.template_create_open && buttons.action(ui, &texts.cancel) {
            intent = Some(ProjectIntent::CancelTemplateCreate);
        }
        if buttons.action(ui, &texts.new_template) {
            intent = Some(ProjectIntent::BeginTemplateCreate);
        }
    });
    intent
}

/// The own template being made: what it starts from, its name with Save, its description.
pub fn own(ui: &mut egui::Ui, pen: &Pen, texts: &ProjectTexts, session: &mut ProjectSession) -> Option<ProjectIntent> {
    let can_save = !session.template_draft_name.trim().is_empty() && !session.selected_template_id.is_empty();
    let base_name = session.selected_template_name();
    let mut save = false;
    pen.section(ui, &texts.new_template, |ui| {
        pen.hint(ui, &texts.template_base.replace("{base}", &base_name));
        pen.row(
            ui,
            &texts.template_name_label,
            |ui, row| {
                row.input(ui, &mut session.template_draft_name, row.width, &texts.template_name_hint);
            },
            |ui, buttons| save = buttons.primary(ui, &texts.save, can_save),
        );
        pen.area(ui, &mut session.template_draft_description, &texts.short_description_hint);
    });
    save.then_some(ProjectIntent::SaveUserTemplate)
}
