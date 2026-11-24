//! A group of the Project and Settings forms (user 2026-11-24: "formu postavke podijeli na kocke
//! ovisno o grupama"): the locations: the projects, the global Ingest bases (QNC-AI, apart from the
//! projects') and export. Each a row of the settings style block (`qnc_settings_block::Pen`): the
//! path as typed, Choose, and while open its browser with Select and Cancel
//! (`qnc-location-field-view`). A typed path is kept on Enter or when the field is left. It gives
//! what the user asked as the Project session's intents; it writes nothing itself.

use qnc_desktop_theme::Theme;
use qnc_project_session::{LocationTarget, ProjectIntent, ProjectSession};
use qnc_project_texts::ProjectTexts;
use qnc_settings_block::Pen;

/// Draws the group, its title on the left when `titled`: the projects' and the export folder (the
/// Ingest bases are the System group's, user 2026-11-24); gives the intents.
pub fn show(ui: &mut eframe::egui::Ui, pen: &Pen, theme: &Theme, texts: &ProjectTexts, session: &mut ProjectSession, titled: bool) -> Vec<ProjectIntent> {
    let title = titled.then(|| qnc_ui_text::text("project.locations_title"));
    show_targets(ui, pen, theme, texts, session, title, &[LocationTarget::ProjectsRoot, LocationTarget::ExportDir])
}

/// Draws the location rows `targets` under `title`; gives the intents.
pub fn show_targets(ui: &mut eframe::egui::Ui, pen: &Pen, theme: &Theme, texts: &ProjectTexts, session: &mut ProjectSession, title: Option<&str>, targets: &[LocationTarget]) -> Vec<ProjectIntent> {
    if let Some(title) = title {
        pen.heading(ui, title);
    }
    let mut intents = Vec::new();
    for target in targets {
        intents.extend(row(ui, pen, theme, texts, session, *target));
    }
    intents
}

/// One location: its row and, while open, its browser.
fn row(ui: &mut eframe::egui::Ui, pen: &Pen, theme: &Theme, texts: &ProjectTexts, session: &mut ProjectSession, target: LocationTarget) -> Vec<ProjectIntent> {
    let bases_label = qnc_ui_text::text("project.bases_root_label").to_string();
    let share_label = qnc_ui_text::text("project.ai_share_label").to_string();
    let (label, field, id, height) = match target {
        LocationTarget::AiShare => (&share_label, &mut session.ai_share, "ai_share", 150.0),
        LocationTarget::ProjectsRoot => (&texts.projects_root_label, &mut session.projects_root, "project_root", 170.0),
        LocationTarget::ExportDir => (&texts.export_directory_label, &mut session.export_dir, "export_dir", 150.0),
        LocationTarget::BasesRoot => (&bases_label, &mut session.bases_root, "bases_root", 150.0),
    };
    let (mut typed, mut pick, mut left) = (false, false, false);
    pen.row(
        ui,
        label,
        |ui, row| {
            let response = row.edit(ui, &mut field.text, row.width, &texts.path_hint);
            typed = response.changed();
            left = response.lost_focus();
        },
        |ui, buttons| pick = buttons.primary(ui, &texts.choose, true),
    );
    let labels = qnc_location_field_view::FieldLabels {
        sources: &texts.sources,
        up: &texts.up,
        disks: &texts.disks,
        internet: &texts.internet,
        empty_folder: &texts.empty_folder,
        empty_disks: &texts.empty_disks,
        empty_lan: &texts.empty_lan,
        empty_internet: &texts.empty_internet,
        confirm: &texts.confirm,
        cancel: &texts.cancel,
    };
    let answer = field.open.then(|| qnc_location_field_view::show_browser(ui, field, id, height, &labels, theme)).flatten();
    if typed {
        session.location_typed(target);
        session.status = texts.settings_changed.clone();
    }
    let mut intents = Vec::new();
    intents.extend(left.then_some(ProjectIntent::CommitLocation { target }));
    intents.extend(pick.then_some(ProjectIntent::PickLocation { target }));
    intents.extend(answer.map(|answer| ProjectIntent::AnswerLocation { target, answer }));
    intents
}
