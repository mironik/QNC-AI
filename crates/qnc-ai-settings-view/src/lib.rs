//! The AI tab of QNC-AI's Settings form (user 2026-11-24: "dodaj jos AI tab i Export tab i u njih
//! premjesti sadrzaj tih tabova"; before, a window of its own opened from the Settings form): only
//! what goes where. On the left the tab's menu, on the right the group chosen. Every group is a
//! block of its own that keeps its own draft and writes its own file:
//! - AI templates (`qnc-ai-templates-view`);
//! - apart under a line: AI engines (`qnc-ai-engines-view`), AI server and LLM
//!   (`qnc-ai-server-view`), shot labels
//!   (`qnc-shot-labels-view`).
//!
//! Everything is drawn by the settings style block (`qnc_settings_block::Pen`).

use std::path::Path;

use eframe::egui;
use qnc_settings_block::Pen;
use qnc_wizard_text::text;

/// The group the right side shows (the shared AI folder is a general setting, user 2026-11-24:
/// "tome je mjesto u tabu Opcenito"; it is the General tab's).
#[derive(Clone, Copy, PartialEq)]
enum Group {
    Templates,
    Engines,
    Server,
    Labels,
}

/// The groups under the line, in the menu's order, with the text code of each.
const SETTINGS: [(Group, &str); 3] = [(Group::Engines, "ai_settings.engines"), (Group::Server, "ai_settings.server"), (Group::Labels, "ai_settings.labels")];

/// The group chosen, kept by the tab.
fn chosen(ctx: &egui::Context, root: &Path) -> (egui::Id, Option<Group>) {
    let id = egui::Id::new(("qnc-ai-settings-view", root.to_string_lossy().to_string()));
    (id, ctx.data_mut(|data| data.get_temp::<Option<Group>>(id)).flatten())
}

/// The tab's menu: the AI templates, then apart under a line the other groups.
pub fn menu(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let (id, mut group) = chosen(ui.ctx(), root);
    if qnc_ai_templates_view::menu(ui, root, pen, group == Some(Group::Templates)) {
        group = Some(Group::Templates);
    }
    pen.menu_rule(ui);
    for (each, code) in SETTINGS {
        if pen.menu_item(ui, group == Some(each), &text(code), "") {
            group = Some(each);
        }
    }
    ui.ctx().data_mut(|data| data.insert_temp(id, group));
}

/// The group chosen in the menu.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    match chosen(ui.ctx(), root).1 {
        None => pen.hint(ui, &text("ai_settings.choose")),
        Some(Group::Templates) => qnc_ai_templates_view::panel(ui, root, pen),
        Some(Group::Engines) => qnc_ai_engines_view::panel(ui, root, pen),
        Some(Group::Server) => pen.section(ui, &text("ai_settings.server"), |ui| qnc_ai_server_view::panel(ui, root, pen)),
        Some(Group::Labels) => qnc_shot_labels_view::panel(ui, root, pen),
    }
}
