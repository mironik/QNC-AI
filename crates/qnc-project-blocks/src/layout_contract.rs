use qnc_contracts::validate_ui_layout_contract_json;
use qnc_keyboard_shortcut::ShortcutCatalog;
use serde::Deserialize;

const SHELL_LAYOUT_JSON: &str = include_str!("../../../contracts/ui/shell.layout.json");
const PROJECT_LAYOUT_JSON: &str = include_str!("../../../contracts/ui/project.layout.json");
/// The blocks of QNC-AI's Settings form, in their order (user 2026-11-23).
const SETTINGS_LAYOUT_JSON: &str = include_str!("../../../contracts/ui/settings.layout.json");
const KEYBOARD_SHORTCUTS_JSON: &str =
    include_str!("../../../contracts/qnc-keyboard-shortcuts.json");

#[derive(Debug, Clone)]
pub struct AppContracts {
    pub shell: ShellLayoutContract,
    pub project: ProjectLayoutContract,
    pub shortcuts: ShortcutCatalog,
    pub project_open_hint: Option<String>,
    /// The one desktop theme, for the public pieces the board places.
    pub theme: qnc_desktop_theme::Theme,
    /// The blocks of the Settings form, in the order its layout names them.
    pub settings_blocks: Vec<SettingsBlock>,
}

impl AppContracts {
    pub fn load_embedded() -> Result<Self, String> {
        Self::load_with(None)
    }

    /// As [`load_embedded`](Self::load_embedded), with the measures of the form's look put into the
    /// project layout (user 2026-11-16: the look editor of every form).
    pub fn load_with(look: Option<&qnc_form_look::FormLook>) -> Result<Self, String> {
        validate_layout_json("contracts/ui/shell.layout.json", SHELL_LAYOUT_JSON)?;
        validate_layout_json("contracts/ui/project.layout.json", PROJECT_LAYOUT_JSON)?;

        let shell = serde_json::from_str::<ShellLayoutContract>(SHELL_LAYOUT_JSON)
            .map_err(|error| format!("shell layout parse failed: {error}"))?;
        // The texts of the contract are codes, in the language of this computer (qnc-ui-text).
        let mut project = qnc_ui_text::resolve_contract(PROJECT_LAYOUT_JSON)?;
        if let Some(look) = look {
            look.apply(&mut project)?;
        }
        let project = serde_json::from_value::<ProjectLayoutContract>(project)
            .map_err(|error| format!("project layout parse failed: {error}"))?;
        let shortcuts = ShortcutCatalog::from_json_str(KEYBOARD_SHORTCUTS_JSON)?;
        let project_open_hint = shortcuts.chord_hint("project", "project_open_selected");
        if project_open_hint.is_none() {
            return Err("keyboard catalog missing project_open_selected binding".to_string());
        }
        validate_metrics(&shell, &project)?;
        let desktop = serde_json::from_str::<qnc_desktop_theme::ShellLayoutContract>(SHELL_LAYOUT_JSON)
            .map_err(|error| format!("shell layout parse failed: {error}"))?;
        let theme = qnc_desktop_theme::Theme::from_contract(&desktop);
        let settings_blocks = settings_blocks(SETTINGS_LAYOUT_JSON)?;
        Ok(Self { shell, project, shortcuts, project_open_hint, theme, settings_blocks })
    }
}

fn validate_metrics(
    shell: &ShellLayoutContract,
    project: &ProjectLayoutContract,
) -> Result<(), String> {
    if shell.shell_metrics.footer_height <= 0.0
        || shell.shell_metrics.workspace_status_height <= 0.0
        || shell.shell_metrics.workspace_footer_columns == 0
    {
        return Err("shell metrics must be positive".to_string());
    }
    if shell.theme_metrics.font_ui <= 0.0
        || shell.theme_metrics.font_timecode <= 0.0
        || shell.theme_metrics.chrome_row_height <= 0.0
        || shell.theme_metrics.chrome_control_height <= 0.0
    {
        return Err("theme metrics must be positive".to_string());
    }
    if project.left_project_list.ready_text.trim().is_empty() {
        return Err("project list ready_text is required".to_string());
    }
    if project.right_settings_panel.field_min_width <= 0.0 {
        return Err("project settings field_min_width must be positive".to_string());
    }
    Ok(())
}

fn validate_layout_json(name: &str, contents: &str) -> Result<(), String> {
    let report = validate_ui_layout_contract_json(name, contents);
    if report.is_ok() {
        Ok(())
    } else {
        Err(report.errors.join("; "))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShellLayoutContract {
    pub layout_id: String,
    pub shell_metrics: ShellMetrics,
    pub theme_metrics: ThemeMetrics,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShellMetrics {
    pub footer_height: f32,
    pub workspace_status_height: f32,
    pub workspace_footer_columns: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeMetrics {
    pub font_ui: f32,
    pub font_timecode: f32,
    pub chrome_row_height: f32,
    pub chrome_pad_x: i8,
    pub chrome_pad_y: i8,
    pub chrome_control_height: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ThemeColors {
    pub bg: [u8; 3],
    pub surface: [u8; 3],
    pub raised: [u8; 3],
    pub border: [u8; 3],
    pub text: [u8; 3],
    pub muted: [u8; 3],
    pub accent: [u8; 3],
    pub focus: [u8; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectLayoutContract {
    pub layout_id: String,
    pub board: BoardMetrics,
    pub left_project_list: ProjectListMetrics,
    pub right_settings_panel: SettingsPanelMetrics,
    /// The checkboxes of the AI box; the filmstrip one decides `artifacts.filmstrip`
    /// (user rule 2026-09-30).
    pub ai_switches: Vec<qnc_settings_switches::Switch>,
    pub pts_slots: PtsSlots,
    /// The Advanced settings as a schema (audit 2026-10-10: they lived as code here).
    pub advanced_settings: AdvancedSettings,
    /// The texts of the board (audit 2026-10-10: they lived as code here).
    pub texts: ProjectTexts,
}

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

/// The one desktop board (`qnc-board`): Project fills the body and the right panel.
pub type BoardMetrics = qnc_board::BoardSizes;

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectListMetrics {
    pub panel_pad: f32,
    pub title: String,
    pub title_row_height: f32,
    pub below_title_pad: f32,
    pub row_height: f32,
    pub row_gap: f32,
    pub delete_column_width: f32,
    pub column_gap: f32,
    pub empty_text: String,
    pub ready_text: String,
}

/// The measures of the settings panel (drawn by the piece `qnc-settings-rows`).
pub use qnc_settings_rows::PanelMetrics as SettingsPanelMetrics;

#[derive(Debug, Clone, Deserialize)]
pub struct PtsSlots {
    pub fixed_order: Vec<String>,
    pub scroll_order: Vec<String>,
}

#[cfg(test)]
mod look_tests {
    use super::*;

    #[test]
    fn the_looks_of_projects_and_settings_point_only_at_measures_the_contract_has() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for form in ["projects", "settings"] {
            let look = qnc_form_look::FormLook::load(&root, form, &["window_width"]).unwrap();
            AppContracts::load_with(Some(&look)).unwrap_or_else(|error| panic!("{form}: {error}"));
        }
    }
}

/// A block of the Settings form (`contracts/ui/settings.layout.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsBlock {
    /// The projects', the Ingest bases' and the export folders.
    Locations,
    /// The ingest orchestrator of each form (`qnc-ingest-orchestrator-view`).
    IngestOrchestrator,
    /// The AI settings of this computer, with their own window (`qnc-ai-settings-view`): AI
    /// templates, AI engines, the AI server and LLM, the shared AI folder, the shot labels.
    Ai,
    /// The card readers on the network (`qnc-card-readers-view`).
    CardReaders,
    /// The tray of this computer: what its menu shows, the form each mode opens
    /// (`qnc-tray-settings-view`).
    Tray,
    /// The templates and the Advanced settings of a project.
    Templates,
    /// The export templates of this computer, with their own window (`qnc-export-templates-view`).
    Export,
}

/// The blocks a Settings layout names; a name no block has is an error, never skipped.
fn settings_blocks(json: &str) -> Result<Vec<SettingsBlock>, String> {
    #[derive(Deserialize)]
    struct Layout {
        blocks: Vec<SettingsBlock>,
    }
    serde_json::from_str::<Layout>(json).map(|layout| layout.blocks).map_err(|error| format!("settings layout parse failed: {error}"))
}

#[cfg(test)]
mod settings_layout_tests {
    use super::*;

    #[test]
    fn the_settings_form_puts_its_blocks_in_the_layouts_order_and_an_unknown_one_is_an_error() {
        let blocks = settings_blocks(SETTINGS_LAYOUT_JSON).unwrap();
        assert_eq!(blocks.first(), Some(&SettingsBlock::Locations));
        assert!(blocks.contains(&SettingsBlock::CardReaders) && blocks.contains(&SettingsBlock::Templates));
        assert!(settings_blocks(r#"{"blocks":["moon"]}"#).is_err());
    }
}
