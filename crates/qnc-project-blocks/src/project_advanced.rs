use eframe::egui::{self, RichText, Sense};
use qnc_settings_path::{path_string, set_string_path};
use serde_json::Value;

use crate::{
    layout_contract::{AdvancedSection, AdvancedSettings, SettingsPanelMetrics},
};
use qnc_desktop_theme::Theme;
use qnc_settings_block::Pen;

pub enum ApplicationSelectionAction {
    Choose {
        priority_group: String,
        application_id: Option<String>,
    },
}

pub struct AdvancedDraft<'a> {
    pub draft_settings: &'a mut Value,
    pub export_preset_draft_name: &'a mut String,
    pub applications: &'a qnc_application_selection::ApplicationSelectionView,
    /// The keyboard presets of the catalog (id, name) and the one used when the project
    /// chooses none: a choice of the project, not of its template (user 2026-10-09).
    pub keyboard_presets: &'a [(String, String)],
    pub keyboard_default: &'a str,
}

/// The Advanced bar: opens and closes everything technical of the panel (user 2026-10-27:
/// the journalist sees the story data and the template; the rest only under Advanced).
pub fn header(ui: &mut egui::Ui, content_w: f32, advanced: &AdvancedSettings, t: &Theme, open: &mut bool) {
    ui.set_max_width(content_w);
    let summary = egui::Frame::NONE
        .fill(t.surface_alt)
        .stroke(egui::Stroke::new(1.0, t.border))
        .inner_margin(egui::Margin::symmetric(8, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&advanced.title).size(13.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(if *open { "▲" } else { "▼" }).size(11.0).color(t.text_muted));
                });
            });
        });
    if summary.response.interact(Sense::click()).clicked() {
        *open = !*open;
    }
}

/// The Advanced settings: its sections come from the layout contract, and everything in them is
/// drawn by the settings style block (`qnc_settings_block::Pen`, user 2026-11-23: "prebaci i
/// napredne postavke projekta na kocku"): each section a section of the Settings form, each field a
/// row; this board only composes the blocks that are more than fields (keyboard preset, export
/// preset, interface language, workflow applications).
pub fn show(
    ui: &mut egui::Ui,
    content_w: f32,
    settings: &SettingsPanelMetrics,
    advanced: &AdvancedSettings,
    shell: &Theme,
    draft: AdvancedDraft<'_>,
    application_action: &mut Option<ApplicationSelectionAction>,
) -> bool {
    let pen = Pen::new(settings, shell, content_w);
    let mut draft = draft;
    let mut changed = false;
    for section in &advanced.sections {
        pen.section(ui, &section.title, |ui| {
            changed |= section_body(ui, &pen, section, advanced, shell, &mut draft, application_action);
        });
    }
    changed
}

fn section_body(
    ui: &mut egui::Ui,
    pen: &Pen,
    section: &AdvancedSection,
    advanced: &AdvancedSettings,
    shell: &Theme,
    draft: &mut AdvancedDraft<'_>,
    application_action: &mut Option<ApplicationSelectionAction>,
) -> bool {
    let mut changed = false;
    match section.block.as_deref() {
        Some("keyboard_preset") => {
            let path = "keyboard_shortcuts.active_preset";
            let current = path_string(draft.draft_settings, path, draft.keyboard_default);
            if let Some(chosen) = pen.choice(ui, &section.label, "pts_kbd_preset", &current, draft.keyboard_presets) {
                set_string_path(draft.draft_settings, path, chosen);
                changed = true;
            }
        }
        Some("export_preset") => {
            let mut presets: Vec<_> = qnc_export_preset::presets(draft.draft_settings).into_iter().map(|preset| (preset.id, preset.name)).collect();
            presets.push((qnc_export_preset::MANUAL_PRESET_ID.to_string(), advanced.texts.manual_preset.clone()));
            let current = path_string(draft.draft_settings, "export.preset", qnc_export_preset::MANUAL_PRESET_ID);
            let selected = pen.choice(ui, &section.label, "pts_export_preset", &current, &presets).unwrap_or(current);
            changed |= qnc_export_preset::apply_preset(draft.draft_settings, &selected);
        }
        Some("applications") => applications(ui, advanced, shell, draft, application_action),
        Some(qnc_language_choice::SECTION) => language(ui, pen, &section.label),
        _ => {}
    }
    changed |= pen.fields(ui, draft.draft_settings, &section.fields, &advanced.options);
    if let Some(then) = section.then.as_ref().filter(|then| then.when.holds(draft.draft_settings)) {
        changed |= pen.fields(ui, draft.draft_settings, &then.fields, &advanced.options);
        if then.save_preset {
            let mut save = false;
            let can_save = !draft.export_preset_draft_name.trim().is_empty();
            pen.row(
                ui,
                "",
                |ui, row| {
                    row.input(ui, draft.export_preset_draft_name, row.width, &advanced.texts.new_preset_hint);
                },
                |ui, buttons| save = buttons.primary(ui, &advanced.texts.save_preset, can_save),
            );
            if save && qnc_export_preset::save_custom_preset(draft.draft_settings, draft.export_preset_draft_name) {
                draft.export_preset_draft_name.clear();
                changed = true;
            }
        }
    }
    changed
}

/// The interface language of this computer (the languages QNC has dictionaries for, written to
/// this computer's setting by `qnc-ui-text`, as the shared block `qnc-language-choice` does), in a
/// row of the style block; while it is not the language QNC runs in, a note that it applies on restart.
fn language(ui: &mut egui::Ui, pen: &Pen, label: &str) {
    let Some(root) = qnc_dev_diagnostics::locate_qnc_root() else { return };
    let chosen = qnc_ui_text::chosen_language(&root);
    if let Some(language) = pen.choice(ui, label, "qnc_ui_language", &chosen, &qnc_ui_text::languages()) {
        if let Err(error) = qnc_ui_text::choose_language(&root, &language) {
            pen.said(ui, &Some(Err(error)));
        }
    }
    let running = qnc_ui_text::host().map(|texts| texts.language.clone()).unwrap_or_default();
    if qnc_ui_text::chosen_language(&root) != running {
        pen.hint(ui, qnc_ui_text::text("project.language_after_restart"));
    }
}

fn applications(
    ui: &mut egui::Ui,
    advanced: &AdvancedSettings,
    t: &Theme,
    draft: &AdvancedDraft<'_>,
    application_action: &mut Option<ApplicationSelectionAction>,
) {
    let applications = draft.applications;
    ui.spacing_mut().item_spacing.y = 6.0;
    let columns: Vec<_> = applications
        .groups
        .iter()
        .map(|group| {
            let mut options: Vec<_> = group
                .choices
                .iter()
                .map(|choice| qnc_ui_kit::OptionItem {
                    id: Some(choice.application_id.clone()),
                    label: choice.label.clone(),
                    selected: choice.selected,
                })
                .collect();
            if !applications.required_groups.contains(&group.priority_group) {
                options.push(qnc_ui_kit::OptionItem {
                    id: None,
                    label: advanced.texts.no_selection.clone(),
                    selected: group.no_selection,
                });
            }
            qnc_ui_kit::OptionColumn {
                id: group.priority_group.clone(),
                label: advanced.texts.group.replace("{group}", &group.priority_group),
                options,
            }
        })
        .collect();
    let style = qnc_ui_kit::OptionColumnsStyle {
        font_size: t.font_ui,
        text: t.text,
        min_column_width: 160.0,
        column_gap: 12.0,
        row_gap: 6.0,
    };
    if let Some(selected) = qnc_ui_kit::show_option_columns(ui, "project_workflow_groups", &columns, &style) {
        *application_action = Some(ApplicationSelectionAction::Choose {
            priority_group: selected.column_id,
            application_id: selected.option_id,
        });
    }
    if applications.loading {
        ui.label(&advanced.texts.loading_catalog);
    }
    if let Some(error) = &applications.error {
        ui.label(error);
    }
}
