//! A block of QNC-AI's Settings form: the tray of this computer (user 2026-11-23: "u postavkama
//! trebaju biti checkboxovi za postavljanje onoga što će se vidjeti u trayu"; "to možemo imati kao
//! postavku u Postavkama, na taj način ništa nije hardkodirano").
//!
//! - a box for every item the tray's menu may show (`qnc-tray-menu`), written at once; Quit is
//!   always there;
//! - the form each mode opens after the ingest Wizard and on Open application (`qnc-tray-mode`):
//!   one of the application catalog's forms (`data/application-catalog.json`) or none, with its
//!   arguments; written on Save.
//!
//! What the tray shows changes at its next look (it reads the setting again).

use std::collections::BTreeSet;
use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_settings_block::{said, title};
use qnc_settings_rows::PanelMetrics;
use qnc_tray_mode::{Forms, Opens};
use qnc_wizard_text::text;

#[derive(Clone, Default)]
struct Draft {
    hidden: BTreeSet<String>,
    forms: Forms,
    /// The arguments of each mode's form as typed (space between arguments).
    args: Vec<String>,
    /// The catalog's forms: (program, label).
    apps: Vec<(String, String)>,
    menu_said: Option<Result<String, String>>,
    forms_said: Option<Result<String, String>>,
}

impl Draft {
    fn read(root: &Path) -> Self {
        let mut draft = Self::default();
        match qnc_tray_menu::hidden(root) {
            Ok(hidden) => draft.hidden = hidden,
            Err(error) => draft.menu_said = Some(Err(error)),
        }
        match qnc_tray_mode::forms(root) {
            Ok(forms) => draft.forms = forms,
            Err(error) => draft.forms_said = Some(Err(error)),
        }
        draft.args = qnc_tray_mode::ALL.iter().map(|mode| draft.forms.of(*mode).map(|opens| opens.args.join(" ")).unwrap_or_default()).collect();
        match apps(root) {
            Ok(apps) => draft.apps = apps,
            Err(error) => draft.forms_said = Some(Err(error)),
        }
        draft
    }
}

/// The forms of the application catalog of this QNC: their program and their name.
fn apps(root: &Path) -> Result<Vec<(String, String)>, String> {
    let file = root.join("data").join("application-catalog.json");
    let text = std::fs::read_to_string(&file).map_err(|error| format!("{}: {error}", file.display()))?;
    let catalog: qnc_application_catalog::Catalog = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", file.display()))?;
    Ok(catalog.applications.into_iter().map(|app| (app.standalone_executable, app.label)).collect())
}

/// Draws the block `width` wide, in the rows of the settings panel.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-tray-settings-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    menu(ui, root, metrics, theme, &mut draft);
    forms(ui, root, width, metrics, theme, &mut draft);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}

/// A box for every item of the tray's menu; a change is written at once.
fn menu(ui: &mut egui::Ui, root: &Path, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    title(ui, "settings_tray.menu_title", "settings_tray.menu_hint", metrics, theme);
    let mut changed = false;
    for item in qnc_tray_menu::ALL {
        let mut shown = !item.may_hide() || !draft.hidden.contains(item.code());
        let label = RichText::new(text(item.label())).size(metrics.label_font_size).color(theme.text);
        if qnc_ui_kit::check_box(ui, &mut shown, label, item.may_hide()).changed() {
            if shown {
                draft.hidden.remove(item.code());
            } else {
                draft.hidden.insert(item.code().to_string());
            }
            changed = true;
        }
    }
    if changed {
        draft.menu_said = Some(qnc_tray_menu::set_hidden(root, &draft.hidden).map(|()| text("machine.saved")));
    }
    said(ui, &draft.menu_said, theme);
}

/// The form each mode opens: one of the catalog's forms or none, and its arguments; Save writes them.
fn forms(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme, draft: &mut Draft) {
    title(ui, "settings_tray.forms_title", "settings_tray.forms_hint", metrics, theme);
    let none = text("settings_tray.form_none");
    for (index, mode) in qnc_tray_mode::ALL.into_iter().enumerate() {
        let program = draft.forms.of(mode).map(|opens| opens.program.clone()).unwrap_or_default();
        let shown = draft.apps.iter().find(|(app, _)| *app == program).map_or_else(|| if program.is_empty() { none.clone() } else { program.clone() }, |(_, label)| label.clone());
        let mut chosen = None;
        qnc_settings_rows::inline_row(
            ui,
            width,
            &text(mode.label()),
            metrics,
            theme,
            |ui, field_width| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt(("qnc-tray-form", index)).width((field_width * 0.45).max(120.0)).selected_text(shown).show_ui(ui, |ui| {
                        if ui.selectable_label(program.is_empty(), &none).clicked() {
                            chosen = Some(String::new());
                        }
                        for (app, label) in &draft.apps {
                            if ui.selectable_label(*app == program, label).clicked() {
                                chosen = Some(app.clone());
                            }
                        }
                    });
                    if let Some(args) = draft.args.get_mut(index) {
                        ui.add(qnc_ui_kit::text_field(args).desired_width((field_width * 0.5).max(80.0)).hint_text(text("settings_tray.form_args")));
                    }
                });
            },
            |_, _| {},
        );
        if let Some(program) = chosen {
            let args = draft.args.get(index).map(|args| args.split_whitespace().map(String::from).collect()).unwrap_or_default();
            draft.forms.set(mode, Some(Opens { program, args }));
        }
    }
    let mut save = false;
    qnc_settings_rows::trailing_actions(ui, width, metrics, |ui| save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked());
    if save {
        for (index, mode) in qnc_tray_mode::ALL.into_iter().enumerate() {
            let opens = draft.forms.of(mode).cloned().map(|opens| Opens { args: draft.args.get(index).map(|args| args.split_whitespace().map(String::from).collect()).unwrap_or_default(), ..opens });
            draft.forms.set(mode, opens);
        }
        draft.forms_said = Some(qnc_tray_mode::set_forms(root, &draft.forms).map(|()| text("machine.saved")));
    }
    said(ui, &draft.forms_said, theme);
}
