//! A group of QNC-AI's AI settings (user 2026-11-23: "formu postavke podijeli na kocke ovisno o
//! grupama"): the shared AI folder of this computer, where the AI models are for QNC and QNC-AI
//! (`qnc_ai_catalog::set_share`, the computer's `ai-share.json`). It keeps its own draft and draws
//! with the settings style block (`qnc_settings_block::Pen`).

use std::path::Path;

use eframe::egui;
use qnc_settings_block::Pen;
use qnc_wizard_text::text;

#[derive(Clone)]
struct Draft {
    folder: String,
    said: Option<Result<String, String>>,
}

fn id(root: &Path) -> egui::Id {
    egui::Id::new(("qnc-ai-share-view", root.to_string_lossy().to_string()))
}

/// Drops what was typed, so the next draw reads the setting again.
pub fn forget(ctx: &egui::Context, root: &Path) {
    ctx.data_mut(|data| data.remove::<Draft>(id(root)));
}

/// Draws the group: the folder and Save.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id(root))).unwrap_or_else(|| Draft {
        folder: qnc_ai_catalog::chosen_share(root).map(|folder| folder.display().to_string()).unwrap_or_default(),
        said: None,
    });
    pen.section(ui, &text("ai_settings.share"), |ui| {
        pen.text(ui, "ai_settings.share_folder", &mut draft.folder, "");
        pen.hint(ui, &text("ai_settings.share_hint"));
        let mut save = false;
        let can = !draft.folder.trim().is_empty();
        pen.buttons(ui, |ui, buttons| save = buttons.primary(ui, &text("machine.save"), can));
        if save {
            draft.said = Some(qnc_ai_catalog::set_share(root, Path::new(draft.folder.trim())).map(|()| text("machine.saved")));
        }
    });
    pen.said(ui, &draft.said);
    ui.ctx().data_mut(|data| data.insert_temp(id(root), draft));
}
