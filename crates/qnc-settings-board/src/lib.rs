//! The board of QNC-AI's Settings form (user 2026-11-24: "je li postavke monolit" - "da, kreni";
//! "postavke su postavke, ne moze biti vise postavki.. mozes uvesti tabove General, User,
//! Project"): the one form of every setting. On top the tabs, under them on the left the groups
//! of the tab chosen, in the order the Settings layout names them
//! (`contracts/ui/settings.layout.json`), and on the right only the group chosen. General holds
//! the settings of this computer, User those of the user signed in, Project the project
//! templates. Every group is a block of its own that reads and writes its own setting; the board
//! only puts them together and draws with the settings style block (`qnc_settings_block::Pen`).

use std::path::Path;

use eframe::egui;
use qnc_desktop_theme::Theme;
use qnc_project_session::{LocationTarget, ProjectIntent, ProjectSession};
use qnc_project_texts::ProjectTexts;
use qnc_settings_block::Pen;
use qnc_settings_rows::PanelMetrics;
use serde::Deserialize;

const SETTINGS_LAYOUT_JSON: &str = include_str!("../../../contracts/ui/settings.layout.json");

/// A tab of the Settings form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tab {
    /// The settings of this computer, the same for every user.
    General,
    /// The settings of the user signed in.
    User,
    /// The project templates.
    Project,
    /// The AI settings of this computer: their own menu.
    Ai,
    /// The export templates of this computer: their own menu.
    Export,
}

impl Tab {
    fn name(self) -> &'static str {
        match self {
            Tab::General => "settings_tab.general",
            Tab::User => "settings_tab.user",
            Tab::Project => "settings_tab.project",
            Tab::Ai => "settings_tab.ai",
            Tab::Export => "settings_tab.export",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Tab::General => "settings_tab.title_general",
            Tab::User => "settings_tab.title_user",
            Tab::Project => "settings_tab.title_project",
            Tab::Ai => "settings_tab.ai",
            Tab::Export => "settings_tab.export",
        }
    }
}

/// A group of the Settings form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// The projects', the Ingest bases' and the export folders (`qnc-locations-view`).
    Locations,
    /// The ingest orchestrator of each form (`qnc-ingest-orchestrator-view`).
    IngestOrchestrator,
    /// The AI settings, a tab of their own with their own menu (`qnc-ai-settings-view`).
    Ai,
    /// The system's folders of this computer (user 2026-11-24: "Lokaciju IngestBase izdvoji iz
    /// Lokacija i dodaj u Zajednicku AI mapu koju ces preimenovati Sustav"): the shared AI folder,
    /// where the AI models are, and the Ingest bases' folder (`qnc-locations-view` rows).
    System,
    /// The card readers on the network (`qnc-card-readers-view`).
    CardReaders,
    /// The tray: what its menu shows, the form each mode opens (`qnc-tray-settings-view`).
    Tray,
    /// The export templates, a tab of their own with their own menu (`qnc-export-templates-view`).
    Export,
    /// The settings of the user signed in (`qnc-user-settings-view`).
    User,
    /// The project templates and their settings (the caller's, the Project blocks).
    Templates,
}

impl Group {
    /// The text code of its name in the menu.
    fn name(self) -> &'static str {
        match self {
            Group::Locations => "settings_group.locations",
            Group::IngestOrchestrator => "settings_group.ingest_orchestrator",
            Group::Ai => "settings_group.ai",
            Group::System => "settings_group.system",
            Group::CardReaders => "settings_group.card_readers",
            Group::Tray => "settings_group.tray",
            Group::Export => "settings_group.export",
            Group::User => "settings_group.user",
            Group::Templates => "settings_group.templates",
        }
    }

    /// The tab a group belongs to: a setting of the computer is never under User, nor one of
    /// the user under General.
    fn tab(self) -> Tab {
        match self {
            Group::User => Tab::User,
            Group::Templates => Tab::Project,
            Group::Ai => Tab::Ai,
            Group::Export => Tab::Export,
            _ => Tab::General,
        }
    }

    /// Whether the group brings its own menu: alone in its tab, its menu is the tab's menu.
    fn own_menu(self) -> bool {
        matches!(self, Group::Ai | Group::Export)
    }
}

/// The tabs the Settings layout names, in its order, each with its groups.
pub fn tabs() -> Result<Vec<(Tab, Vec<Group>)>, String> {
    tabs_of(SETTINGS_LAYOUT_JSON)
}

fn tabs_of(json: &str) -> Result<Vec<(Tab, Vec<Group>)>, String> {
    #[derive(Deserialize)]
    struct Entry {
        tab: Tab,
        groups: Vec<Group>,
    }
    #[derive(Deserialize)]
    struct Layout {
        tabs: Vec<Entry>,
    }
    let layout = serde_json::from_str::<Layout>(json).map_err(|error| format!("settings layout parse failed: {error}"))?;
    let mut tabs = Vec::new();
    for entry in layout.tabs {
        if let Some(group) = entry.groups.iter().find(|group| group.tab() != entry.tab) {
            return Err(format!("settings layout: the group {group:?} does not belong to the tab {:?}", entry.tab));
        }
        if entry.groups.is_empty() || tabs.iter().any(|(tab, _)| *tab == entry.tab) {
            return Err(format!("settings layout: the tab {:?} is empty or named twice", entry.tab));
        }
        tabs.push((entry.tab, entry.groups));
    }
    Ok(tabs)
}

/// What the board needs from its caller beyond the session: the lists the User tab chooses
/// from and the drawing of the project templates (the Project form's own blocks).
pub struct Caller<'a> {
    pub user: qnc_user_settings_view::Lists<'a>,
    pub templates: &'a mut dyn FnMut(&mut egui::Ui, &mut ProjectSession) -> Vec<ProjectIntent>,
}

/// The Settings form of the Project program's contracts and session: the board, its User tab
/// choosing from the contracts' keyboard presets, its Project tab the Project blocks' templates.
pub fn show_settings(ui: &mut egui::Ui, contracts: &qnc_project_blocks::layout_contract::AppContracts, session: &mut ProjectSession) -> Vec<ProjectIntent> {
    let root = session.component.root.clone();
    let presets = contracts.shortcuts.preset_choices(&contracts.project.advanced_settings.keyboard_preset_order);
    let mut templates = |ui: &mut egui::Ui, session: &mut ProjectSession| {
        let width = ui.available_width() - 16.0;
        qnc_project_blocks::show_templates(ui, width, contracts, session)
    };
    let caller = Caller {
        user: qnc_user_settings_view::Lists { keyboard_presets: &presets, keyboard_default: &contracts.shortcuts.active_preset },
        templates: &mut templates,
    };
    let project = &contracts.project;
    show(ui, &root, &project.right_settings_panel, &contracts.theme, &project.texts, session, caller)
}

/// Draws the board over `ui`: the tabs, the menu of the tab's groups, a line, the group chosen.
/// Gives what the groups asked of the Project session.
pub fn show(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, texts: &ProjectTexts, session: &mut ProjectSession, caller: Caller<'_>) -> Vec<ProjectIntent> {
    let id = egui::Id::new(("qnc-settings-board", root.to_string_lossy().to_string()));
    let tabs = match tabs() {
        Ok(tabs) => tabs,
        Err(error) => {
            Pen::new(metrics, theme, ui.available_width()).said(ui, &Some(Err(error)));
            return Vec::new();
        }
    };
    let (mut tab, mut chosen) = ui.ctx().data_mut(|data| data.get_temp::<(usize, Option<Group>)>(id)).unwrap_or((0, None));
    tab = tab.min(tabs.len() - 1);
    let mut intents = Vec::new();
    // The menu as wide as a row's label and button columns, the group the rest.
    let menu_w = metrics.inline_label_width + metrics.inline_button_width;
    egui::Frame::NONE.fill(theme.bg).inner_margin(egui::Margin::same(metrics.panel_pad as i8)).show(ui, |ui| {
        let names: Vec<String> = tabs.iter().map(|(tab, _)| qnc_wizard_text::text(tab.name())).collect();
        if let Some(clicked) = Pen::new(metrics, theme, ui.available_width()).tabs(ui, &names, tab) {
            tab = clicked;
        }
        let (current, groups) = &tabs[tab];
        if !chosen.is_some_and(|group| groups.contains(&group)) {
            chosen = groups.first().copied();
        }
        let height = ui.available_height();
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(menu_w);
                // A tab of one group with its own menu (AI, Export) has that menu on the left.
                let own = groups.iter().copied().filter(|group| group.own_menu()).find(|_| groups.len() == 1);
                egui::ScrollArea::vertical().id_salt(("qnc-settings-board-menu", root.to_string_lossy().to_string())).max_height(height).auto_shrink([false, false]).show(ui, |ui| {
                    let pen = Pen::new(metrics, theme, menu_w - 8.0);
                    match own {
                        Some(Group::Ai) => qnc_ai_settings_view::menu(ui, root, &pen),
                        Some(_) => qnc_export_templates_view::menu(ui, root, &pen),
                        None => {
                            pen.menu_heading(ui, &qnc_wizard_text::text(current.title()));
                            for group in groups {
                                if pen.menu_item(ui, chosen == Some(*group), &qnc_wizard_text::text(group.name()), "") {
                                    chosen = Some(*group);
                                }
                            }
                        }
                    }
                });
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
                        Some(Group::Ai) => qnc_ai_settings_view::panel(ui, root, &pen),
                        Some(Group::System) => intents.extend(qnc_locations_view::show_targets(ui, &pen, theme, texts, session, Some(&qnc_wizard_text::text("settings_group.system")), &[LocationTarget::AiShare, LocationTarget::BasesRoot])),
                        Some(Group::CardReaders) => qnc_card_readers_view::show(ui, root, &pen),
                        Some(Group::Tray) => qnc_tray_settings_view::show(ui, root, &pen),
                        Some(Group::Export) => qnc_export_templates_view::panel(ui, root, &pen),
                        Some(Group::User) => qnc_user_settings_view::show(ui, root, &pen, &caller.user),
                        Some(Group::Templates) => intents.extend((caller.templates)(ui, session)),
                        None => {}
                    }
                });
            });
        });
    });
    ui.ctx().data_mut(|data| data.insert_temp(id, (tab, chosen)));
    intents
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_board_has_the_tabs_of_the_layout_and_a_group_only_under_its_own_tab() {
        let tabs = tabs().unwrap();
        assert_eq!(tabs.iter().map(|(tab, _)| *tab).collect::<Vec<_>>(), vec![Tab::General, Tab::User, Tab::Project, Tab::Ai, Tab::Export]);
        assert_eq!(tabs[0].1.first(), Some(&Group::Locations));
        assert!(tabs[0].1.contains(&Group::System) && tabs[0].1.contains(&Group::CardReaders) && !tabs[0].1.contains(&Group::Ai) && !tabs[0].1.contains(&Group::Export));
        assert_eq!(tabs[1].1, vec![Group::User]);
        assert_eq!(tabs[2].1, vec![Group::Templates]);
        assert_eq!((tabs[3].1.as_slice(), tabs[4].1.as_slice()), (&[Group::Ai][..], &[Group::Export][..]));
        assert!(tabs_of(r#"{"tabs":[{"tab":"general","groups":["ai"]}]}"#).is_err(), "AI is a tab of its own");
        assert!(tabs_of(r#"{"tabs":[{"tab":"general","groups":["templates"]}]}"#).is_err(), "the templates are not a setting of the computer");
        assert!(tabs_of(r#"{"tabs":[{"tab":"user","groups":["ai"]}]}"#).is_err(), "the AI of the computer is not the user's");
        assert!(tabs_of(r#"{"tabs":[{"tab":"general","groups":["nothing"]}]}"#).is_err());
    }
}
