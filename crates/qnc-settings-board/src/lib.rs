//! The board of QNC-AI's Settings form (user 2026-11-24: "je li postavke monolit" - "da, kreni"):
//! the settings of this computer, not of a project. On the left the groups, in the order the
//! Settings layout names them (`contracts/ui/settings.layout.json`); on the right only the group
//! chosen. Every group is a block of its own that reads and writes its own setting; the board only
//! puts them together and draws with the settings style block (`qnc_settings_block::Pen`). The
//! templates and Advanced settings of a project are not here: they belong to the Project form.

use std::path::Path;

use eframe::egui;
use qnc_desktop_theme::Theme;
use qnc_project_session::{ProjectIntent, ProjectSession};
use qnc_project_texts::ProjectTexts;
use qnc_settings_block::Pen;
use qnc_settings_rows::PanelMetrics;
use serde::Deserialize;

const SETTINGS_LAYOUT_JSON: &str = include_str!("../../../contracts/ui/settings.layout.json");

/// A group of the Settings form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// The projects', the Ingest bases' and the export folders (`qnc-locations-view`).
    Locations,
    /// The ingest orchestrator of each form (`qnc-ingest-orchestrator-view`).
    IngestOrchestrator,
    /// The AI settings, with their own window (`qnc-ai-settings-view`).
    Ai,
    /// The card readers on the network (`qnc-card-readers-view`).
    CardReaders,
    /// The tray: what its menu shows, the form each mode opens (`qnc-tray-settings-view`).
    Tray,
    /// The export templates, with their own window (`qnc-export-templates-view`).
    Export,
}

impl Group {
    /// The text code of its name in the menu.
    fn name(self) -> &'static str {
        match self {
            Group::Locations => "settings_group.locations",
            Group::IngestOrchestrator => "settings_group.ingest_orchestrator",
            Group::Ai => "settings_group.ai",
            Group::CardReaders => "settings_group.card_readers",
            Group::Tray => "settings_group.tray",
            Group::Export => "settings_group.export",
        }
    }
}

/// The groups the Settings layout names, in its order; a name no group has is an error.
pub fn groups() -> Result<Vec<Group>, String> {
    groups_of(SETTINGS_LAYOUT_JSON)
}

fn groups_of(json: &str) -> Result<Vec<Group>, String> {
    #[derive(Deserialize)]
    struct Layout {
        blocks: Vec<Group>,
    }
    serde_json::from_str::<Layout>(json).map(|layout| layout.blocks).map_err(|error| format!("settings layout parse failed: {error}"))
}

/// Draws the board over `ui`: the menu of groups, a line, the group chosen. Gives what the
/// locations asked of the Project session (the only group that keeps its setting there).
pub fn show(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, texts: &ProjectTexts, session: &mut ProjectSession) -> Vec<ProjectIntent> {
    let id = egui::Id::new(("qnc-settings-board", root.to_string_lossy().to_string()));
    let groups = match groups() {
        Ok(groups) => groups,
        Err(error) => {
            Pen::new(metrics, theme, ui.available_width()).said(ui, &Some(Err(error)));
            return Vec::new();
        }
    };
    let mut chosen = ui.ctx().data_mut(|data| data.get_temp::<Option<Group>>(id)).flatten().or_else(|| groups.first().copied());
    let mut intents = Vec::new();
    // The menu as wide as a row's label and button columns, the group the rest.
    let menu_w = metrics.inline_label_width + metrics.inline_button_width;
    let height = ui.available_height();
    egui::Frame::NONE.fill(theme.bg).inner_margin(egui::Margin::same(metrics.panel_pad as i8)).show(ui, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(menu_w);
                let pen = Pen::new(metrics, theme, menu_w);
                pen.menu_heading(ui, &qnc_wizard_text::text("settings_group.title"));
                for group in &groups {
                    if pen.menu_item(ui, chosen == Some(*group), &qnc_wizard_text::text(group.name()), "") {
                        chosen = Some(*group);
                    }
                }
            });
            ui.separator();
            let width = ui.available_width();
            ui.vertical(|ui| {
                ui.set_width(width);
                egui::ScrollArea::vertical().id_salt(("qnc-settings-board-group", root.to_string_lossy().to_string())).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                    let pen = Pen::new(metrics, theme, width - 16.0);
                    match chosen {
                        Some(Group::Locations) => intents.extend(qnc_locations_view::show(ui, &pen, theme, texts, session, true)),
                        Some(Group::IngestOrchestrator) => qnc_ingest_orchestrator_view::show(ui, root, &pen),
                        Some(Group::Ai) => qnc_ai_settings_view::show(ui, root, &pen),
                        Some(Group::CardReaders) => qnc_card_readers_view::show(ui, root, &pen),
                        Some(Group::Tray) => qnc_tray_settings_view::show(ui, root, &pen),
                        Some(Group::Export) => qnc_export_templates_view::show(ui, root, &pen),
                        None => {}
                    }
                });
            });
        });
    });
    ui.ctx().data_mut(|data| data.insert_temp(id, chosen));
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_board_has_the_groups_of_the_layout_in_its_order_and_an_unknown_one_is_an_error() {
        let groups = groups().unwrap();
        assert_eq!(groups.first(), Some(&Group::Locations));
        assert!(groups.contains(&Group::CardReaders) && groups.contains(&Group::Export));
        assert!(groups_of(r#"{"blocks":["templates"]}"#).is_err(), "the project's templates are not a group of this computer's settings");
    }
}
