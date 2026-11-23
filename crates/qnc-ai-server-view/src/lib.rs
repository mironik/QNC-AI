//! A block of QNC-AI's Settings form (user 2026-11-22: "u formi Postavke dodati putanju Externog AI
//! Agenta i LLM modela"; 2026-11-23: "forma postavke se slaže od kocaka"): the external AI agent of
//! this computer, the AI server (`qnc-ai-place`: address, certificate fingerprint, token; an empty
//! address removes it and AI runs here), and the LLM model (`qnc-llm-place`, only kept for now);
//! written on Save. What is typed is a draft of this block until it is written; it is read again
//! from the settings when the block starts.

use std::path::Path;

use eframe::egui;
use qnc_desktop_theme::Theme;
use qnc_settings_block::{field, said, title};
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;

#[derive(Clone, Default)]
struct Draft {
    server: String,
    fingerprint: String,
    token: String,
    llm: String,
    said: Option<Result<String, String>>,
}

impl Draft {
    fn read(root: &Path) -> Self {
        let mut draft = Self::default();
        match qnc_ai_place::server_of(root) {
            Ok(Some(server)) => (draft.server, draft.fingerprint, draft.token) = (server.address, server.fingerprint, server.token),
            Ok(None) => {}
            Err(error) => draft.said = Some(Err(error)),
        }
        match qnc_llm_place::model_of(root) {
            Ok(model) => draft.llm = model.unwrap_or_default(),
            Err(error) => draft.said = Some(Err(error)),
        }
        draft
    }

    fn save(&mut self, root: &Path) {
        let server = (!self.server.trim().is_empty())
            .then(|| qnc_ai_place::AiServer { address: self.server.trim().into(), fingerprint: self.fingerprint.trim().into(), token: self.token.trim().into() });
        let written = qnc_ai_place::set_server_of(root, server.as_ref()).and_then(|()| qnc_llm_place::set_model_of(root, &self.llm));
        self.said = Some(written.map(|()| text("machine.saved")));
    }
}

/// Draws the block `width` wide, in the rows of the settings panel.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-ai-server-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    title(ui, "machine.ai_title", "machine.ai_hint", metrics, theme);
    field(ui, width, "machine.ai_server", &mut draft.server, metrics, theme, false);
    field(ui, width, "machine.ai_fingerprint", &mut draft.fingerprint, metrics, theme, false);
    field(ui, width, "machine.ai_token", &mut draft.token, metrics, theme, true);
    let mut save = false;
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text("machine.llm"),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(&mut draft.llm).desired_width(field_width.max(40.0)));
        },
        |ui, _| save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked(),
    );
    if save {
        draft.save(root);
    }
    said(ui, &draft.said, theme);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}
