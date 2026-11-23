//! A group of QNC-AI's AI settings (user 2026-11-23: "formu postavke podijeli na kocke ovisno o
//! grupama"): the shot labels of this computer (`qnc-shot-labels-edit`): labels, prompts,
//! statements, thresholds, caption rules. It keeps its own draft (read from the list the first time
//! it is drawn, again after [`forget`]) and draws with the settings style block
//! (`qnc_settings_block::Pen`).

mod labels;

use std::path::Path;

use eframe::egui;
use qnc_settings_block::Pen;
use qnc_shot_labels_edit::List;

#[derive(Clone)]
struct Draft {
    list: Result<List, String>,
    new: labels::New,
    said: Option<Result<String, String>>,
}

fn id(root: &Path) -> egui::Id {
    egui::Id::new(("qnc-shot-labels-view", root.to_string_lossy().to_string()))
}

/// Drops what was typed, so the next draw reads the list again.
pub fn forget(ctx: &egui::Context, root: &Path) {
    ctx.data_mut(|data| data.remove::<Draft>(id(root)));
}

/// Draws the group: its sections and rows.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id(root))).unwrap_or_else(|| Draft { list: List::installed(), new: labels::New::default(), said: None });
    match &mut draft.list {
        Ok(list) => {
            if let Some(said) = labels::show(ui, pen, list, &mut draft.new) {
                draft.said = Some(said);
            }
        }
        Err(error) => pen.said(ui, &Some(Err(error.clone()))),
    }
    pen.said(ui, &draft.said);
    ui.ctx().data_mut(|data| data.insert_temp(id(root), draft));
}
