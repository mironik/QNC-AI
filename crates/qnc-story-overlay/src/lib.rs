//! Over the Ingest form (user 2026-11-12: "obrazac Novi projekt tretiraj kao overlay prozor Ingest
//! forme"; "ako projekt postoji onda korisnik može nastaviti dalje ili kreirati novi projekt"):
//! the choice to go on with the story worked on before or make a new one, then QNC's new project
//! sheet (`qnc-newsroom-sheet`, with the template). It draws only and answers what the user chose;
//! the piece `qnc-story-start` holds the state and does the work.

use eframe::egui;
use qnc_newsroom_sheet::{Answer, SheetOverlay};

/// What the user chose over the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    GoOn,
    New,
    Cancel,
    Sheet(Answer),
}

const GO_ON: &str = "new_project_go_on";
const NEW: &str = "new_project_new";
const CANCEL: &str = "new_project_cancel";

/// Draws the choice (when `offer` names a story) or the sheet (when there is one).
pub fn show(
    ctx: &egui::Context,
    theme: &qnc_desktop_theme::Theme,
    catalog: &qnc_keyboard_shortcut::ShortcutCatalog,
    offer: Option<&str>,
    sheet: Option<&mut SheetOverlay>,
) -> Option<Chosen> {
    if let Some(name) = offer {
        use qnc_wizard_text::{fill, text};
        let choice = qnc_choice_box::Choice {
            title: text("new_project.offer_title"),
            message: fill("new_project.offer_message", &[("name", name)]),
            options: vec![
                qnc_choice_box::ChoiceOption::new(GO_ON, fill("new_project.go_on", &[("name", name)]), true),
                qnc_choice_box::ChoiceOption::new(NEW, text("new_project.new"), false),
                qnc_choice_box::ChoiceOption::new(CANCEL, text("new_project.cancel"), false),
            ],
            ..Default::default()
        };
        return match qnc_choice_box::show(ctx, theme, &choice).as_deref() {
            Some(GO_ON) => Some(Chosen::GoOn),
            Some(NEW) => Some(Chosen::New),
            Some(_) => Some(Chosen::Cancel),
            None => None,
        };
    }
    sheet.and_then(|sheet| qnc_newsroom_sheet::show(ctx, sheet, theme, catalog)).map(Chosen::Sheet)
}
