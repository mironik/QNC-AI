//! A block of QNC-AI's Settings form with its own window (user 2026-11-23: "sljedeće neka budu AI
//! postavke, isto samostalna forma"; "formu postavke podijeli na kocke ovisno o grupama"): only the
//! window and what goes where. In the Settings form it is one row and a button; in the window
//! (`qnc-settings-window`) the menu on the left and the group chosen on the right. Every group is a
//! block of its own that keeps its own draft and writes its own file:
//! - AI templates (`qnc-ai-templates-view`);
//! - apart under a line: AI engines (`qnc-ai-engines-view`), AI server and LLM
//!   (`qnc-ai-server-view`), the shared AI folder (`qnc-ai-share-view`), shot labels
//!   (`qnc-shot-labels-view`).
//!
//! Everything is drawn by the settings style block (`qnc_settings_block::Pen`).

use std::path::Path;

use eframe::egui;
use qnc_settings_block::Pen;
use qnc_settings_window::Pane;
use qnc_wizard_text::text;

/// The form name of the window's look.
const LOOK: &str = "ai-settings";

/// The group the right side shows.
#[derive(Clone, Copy, PartialEq)]
enum Group {
    Templates,
    Engines,
    Server,
    Share,
    Labels,
}

/// The groups under the line, in the menu's order, with the text code of each.
const SETTINGS: [(Group, &str); 4] = [(Group::Engines, "ai_settings.engines"), (Group::Server, "ai_settings.server"), (Group::Share, "ai_settings.share"), (Group::Labels, "ai_settings.labels")];

#[derive(Clone, Default)]
struct Window {
    open: bool,
    group: Option<Group>,
    said: Option<Result<String, String>>,
}

/// Every group reads its setting again when the window opens.
fn forget_all(ctx: &egui::Context, root: &Path) {
    qnc_ai_templates_view::forget(ctx, root);
    qnc_ai_engines_view::forget(ctx, root);
    qnc_ai_share_view::forget(ctx, root);
    qnc_shot_labels_view::forget(ctx, root);
}

/// Draws the block's row in the Settings form; its window opens in the middle of the screen.
pub fn show(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let id = egui::Id::new(("qnc-ai-settings-view", root.to_string_lossy().to_string()));
    let mut window = ui.ctx().data_mut(|data| data.get_temp::<Window>(id)).unwrap_or_default();
    pen.title(ui, "ai_settings.title", "ai_settings.hint");
    let mut open = false;
    pen.row(ui, &text("ai_settings.row"), |_, _| {}, |ui, buttons| open = buttons.primary(ui, &text("export_template.open"), true));
    if open && !window.open {
        match qnc_settings_window::open(root, LOOK, &text("ai_settings.window_title")) {
            Ok(()) => {
                forget_all(ui.ctx(), root);
                window = Window { open: true, ..Window::default() };
            }
            Err(error) => window.said = Some(Err(error)),
        }
    }
    if window.open {
        window.open = qnc_settings_window::show(ui.ctx(), root, LOOK, &text("ai_settings.window_title"), pen, |ui, pane, pen| match pane {
            Pane::Menu => menu(ui, root, pen, &mut window),
            Pane::Panel => panel(ui, root, pen, &window),
        });
    } else {
        pen.said(ui, &window.said);
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, window));
}

/// The menu: the AI templates, then apart under a line the other groups.
fn menu(ui: &mut egui::Ui, root: &Path, pen: &Pen, window: &mut Window) {
    if qnc_ai_templates_view::menu(ui, root, pen, window.group == Some(Group::Templates)) {
        window.group = Some(Group::Templates);
    }
    pen.menu_rule(ui);
    for (group, code) in SETTINGS {
        if pen.menu_item(ui, window.group == Some(group), &text(code), "") {
            window.group = Some(group);
        }
    }
}

/// The group chosen on the left.
fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen, window: &Window) {
    match window.group {
        None => pen.hint(ui, &text("ai_settings.choose")),
        Some(Group::Templates) => qnc_ai_templates_view::panel(ui, root, pen),
        Some(Group::Engines) => qnc_ai_engines_view::panel(ui, root, pen),
        Some(Group::Server) => pen.section(ui, &text("ai_settings.server"), |ui| qnc_ai_server_view::panel(ui, root, pen)),
        Some(Group::Share) => qnc_ai_share_view::panel(ui, root, pen),
        Some(Group::Labels) => qnc_shot_labels_view::panel(ui, root, pen),
    }
}
