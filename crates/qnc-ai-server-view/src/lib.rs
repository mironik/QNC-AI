//! A block of QNC-AI's Settings form (user 2026-11-22: "u formi Postavke dodati putanju Externog AI
//! Agenta i LLM modela"; 2026-11-23: "forma postavke se slaže od kocaka"): the external AI agent of
//! this computer, the AI server (`qnc-ai-place`: address, certificate fingerprint, token; an empty
//! address removes it and AI runs here), and the LLM model (`qnc-llm-place`, only kept for now);
//! written on Save. What is typed is a draft of this block until it is written; it is read again
//! from the settings when the block starts.

use std::path::Path;

use eframe::egui;
use qnc_settings_block::Pen;
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

/// The setting in the AI settings window (`qnc-settings-window`), drawn by the settings style block
/// (`qnc_settings_block::Pen`): a row for each value, the token hidden, Save in the button column.
pub fn panel(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let id = egui::Id::new(("qnc-ai-server-view-panel", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    pen.text(ui, "machine.ai_server", &mut draft.server, "");
    pen.text(ui, "machine.ai_fingerprint", &mut draft.fingerprint, "");
    pen.secret(ui, "machine.ai_token", &mut draft.token);
    pen.text(ui, "machine.llm", &mut draft.llm, "");
    pen.hint(ui, &text("machine.ai_hint"));
    let mut save = false;
    pen.buttons(ui, |ui, buttons| save = buttons.primary(ui, &text("machine.save"), true));
    if save {
        draft.save(root);
    }
    pen.said(ui, &draft.said);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}
