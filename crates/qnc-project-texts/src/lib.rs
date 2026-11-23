//! The texts of the Project and Settings forms and the contract of the Project Advanced settings, as
//! the Project layout contract (`contracts/ui/project.layout.json`) has them: data only, so each
//! group of the forms (locations, templates, Advanced settings) is a block of its own that reads
//! them (user 2026-11-24: "formu postavke podijeli na kocke ovisno o grupama").

use serde::Deserialize;


#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTexts {
    pub templates_empty: String,
    pub delete_template_hint: String,
    pub project_name_label: String,
    pub project_name_hint: String,
    pub new_project: String,
    pub create_project: String,
    /// Opens the story sheet of the open project (piece `qnc-newsroom-sheet`).
    pub story_sheet: String,
    pub settings_changed: String,
    pub path_hint: String,
    pub choose: String,
    pub confirm: String,
    pub cancel: String,
    pub new_template: String,
    /// `{base}` is the template the new one starts from.
    pub template_base: String,
    pub template_name_label: String,
    pub template_name_hint: String,
    pub save: String,
    pub short_description_hint: String,
    pub projects_root_label: String,
    pub export_directory_label: String,
    pub sources: String,
    pub up: String,
    pub disks: String,
    pub internet: String,
    pub empty_lan: String,
    pub empty_internet: String,
    pub empty_folder: String,
    pub empty_disks: String,
    pub remove_project_hint: String,
    pub delete_project_question: String,
    pub delete_template_question: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSettings {
    pub title: String,
    pub grid: AdvancedGrid,
    pub options: qnc_settings_form::OptionSets,
    pub sections: Vec<AdvancedSection>,
    /// The keyboard presets shown first, in this order (the others follow by id).
    pub keyboard_preset_order: Vec<String>,
    pub texts: AdvancedTexts,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedGrid {
    pub label_size: f32,
    pub min_cell_width: f32,
    pub gap: f32,
    pub cell_height: f32,
}

/// A section: its fields, or one of the blocks the form composes (`keyboard_preset`,
/// `export_preset`, `applications`), and fields shown while a condition holds.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedSection {
    pub title: String,
    #[serde(default)]
    pub fields: Vec<qnc_settings_form::FieldSpec>,
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub then: Option<AdvancedThen>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedThen {
    pub when: qnc_settings_form::Condition,
    pub fields: Vec<qnc_settings_form::FieldSpec>,
    #[serde(default)]
    pub save_preset: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdvancedTexts {
    pub manual_preset: String,
    pub new_preset_hint: String,
    pub save_preset: String,
    pub no_selection: String,
    pub group: String,
    pub loading_catalog: String,
}
