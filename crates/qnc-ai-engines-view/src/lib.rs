//! A group of QNC-AI's AI settings (user 2026-11-23: "formu postavke podijeli na kocke ovisno o
//! grupama"): the AI engines of this computer (`qnc-ai-catalog-edit`): which transcription engine is
//! the best, the fast and the live one, the threads and the clips at once of each, and the model of
//! every other AI job. It keeps its own draft (read from the catalog the first time it is drawn,
//! again after [`forget`]) and draws with the settings style block (`qnc_settings_block::Pen`).

mod engines;

use std::path::Path;

use eframe::egui;
use qnc_ai_catalog_edit::Catalog;
use qnc_settings_block::Pen;

#[derive(Clone)]
struct Draft {
    catalog: Result<Catalog, String>,
    said: Option<Result<String, String>>,
}

fn id(root: &Path) -> egui::Id {
    egui::Id::new(("qnc-ai-engines-view", root.to_string_lossy().to_string()))
}

/// Drops what was typed, so the next draw reads the catalog again.
pub fn forget(ctx: &egui::Context, root: &Path) {
    ctx.data_mut(|data| data.remove::<Draft>(id(root)));
}

/// Draws the group: its section and its rows.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id(root))).unwrap_or_else(|| Draft { catalog: Catalog::installed(), said: None });
    match &mut draft.catalog {
        Ok(catalog) => {
            if let Some(said) = engines::show(ui, root, pen, catalog) {
                draft.said = Some(said);
            }
        }
        Err(error) => pen.said(ui, &Some(Err(error.clone()))),
    }
    pen.said(ui, &draft.said);
    ui.ctx().data_mut(|data| data.insert_temp(id(root), draft));
}
