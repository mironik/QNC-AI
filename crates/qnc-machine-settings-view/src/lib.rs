//! The settings of this computer in QNC-AI Settings (user 2026-11-22: "u formi Postavke dodati
//! putanju Externog AI Agenta i LLM modela, te putanju importa kroz čitač kartica spojen na
//! udaljenoj lokaciji"), drawn with the settings' own rows (`qnc-settings-rows`):
//!
//! - the external AI agent: the AI server of this computer (`qnc-ai-place`: address, certificate
//!   fingerprint, token; an empty address removes it and AI runs here) and the LLM model
//!   (`qnc-llm-place`, only kept for now); written on Save;
//! - the card readers on the network (`qnc-card-readers`): each a name and the shared folder,
//!   written at once on Add and Remove; Ingest shows each under LAN.
//!
//! What is typed is a draft of this view until it is written; what is shown is read again from
//! the settings when the view starts.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_card_readers::CardReader;
use qnc_desktop_theme::Theme;
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;

#[derive(Clone, Default)]
struct Drafts {
    server: String,
    fingerprint: String,
    token: String,
    llm: String,
    readers: Vec<CardReader>,
    name: String,
    path: String,
    ai_said: Option<Result<String, String>>,
    readers_said: Option<Result<String, String>>,
}

impl Drafts {
    fn read(root: &Path) -> Self {
        let mut drafts = Self::default();
        match qnc_ai_place::server_of(root) {
            Ok(Some(server)) => (drafts.server, drafts.fingerprint, drafts.token) = (server.address, server.fingerprint, server.token),
            Ok(None) => {}
            Err(error) => drafts.ai_said = Some(Err(error)),
        }
        match qnc_llm_place::model_of(root) {
            Ok(model) => drafts.llm = model.unwrap_or_default(),
            Err(error) => drafts.ai_said = Some(Err(error)),
        }
        match qnc_card_readers::readers_of(root) {
            Ok(readers) => drafts.readers = readers,
            Err(error) => drafts.readers_said = Some(Err(error)),
        }
        drafts
    }

    fn save_ai(&mut self, root: &Path) {
        let server = (!self.server.trim().is_empty())
            .then(|| qnc_ai_place::AiServer { address: self.server.trim().into(), fingerprint: self.fingerprint.trim().into(), token: self.token.trim().into() });
        let written = qnc_ai_place::set_server_of(root, server.as_ref()).and_then(|()| qnc_llm_place::set_model_of(root, &self.llm));
        self.ai_said = Some(written.map(|()| text("machine.saved")));
    }

    fn write_readers(&mut self, root: &Path, readers: Vec<CardReader>) {
        match qnc_card_readers::set_readers_of(root, &readers) {
            Ok(()) => {
                self.readers = readers;
                self.readers_said = Some(Ok(text("machine.saved")));
            }
            Err(error) => self.readers_said = Some(Err(error)),
        }
    }
}

/// Draws the settings of this computer `width` wide, in the rows of the settings panel.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-machine-settings-view", root.to_string_lossy().to_string()));
    let mut drafts = ui.ctx().data_mut(|data| data.get_temp::<Drafts>(id)).unwrap_or_else(|| Drafts::read(root));
    ai(ui, root, width, metrics, theme, &mut drafts);
    readers(ui, root, width, metrics, theme, &mut drafts);
    ui.ctx().data_mut(|data| data.insert_temp(id, drafts));
}

fn title(ui: &mut egui::Ui, code: &str, hint: &str, metrics: &PanelMetrics, theme: &Theme) {
    ui.add_space(metrics.section_gap);
    ui.label(RichText::new(text(code).to_uppercase()).size(metrics.label_font_size).strong().color(theme.text_muted));
    ui.label(RichText::new(text(hint)).size(metrics.label_font_size).color(theme.text_muted));
}

fn said(ui: &mut egui::Ui, said: &Option<Result<String, String>>, theme: &Theme) {
    match said {
        Some(Ok(message)) => {
            ui.colored_label(theme.text_muted, message);
        }
        Some(Err(error)) => {
            ui.colored_label(theme.danger, error);
        }
        None => {}
    }
}

fn field(ui: &mut egui::Ui, width: f32, label: &str, value: &mut String, metrics: &PanelMetrics, theme: &Theme, secret: bool) {
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text(label),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(value).desired_width(field_width.max(40.0)).password(secret));
        },
        |_, _| {},
    );
}

fn ai(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme, drafts: &mut Drafts) {
    title(ui, "machine.ai_title", "machine.ai_hint", metrics, theme);
    field(ui, width, "machine.ai_server", &mut drafts.server, metrics, theme, false);
    field(ui, width, "machine.ai_fingerprint", &mut drafts.fingerprint, metrics, theme, false);
    field(ui, width, "machine.ai_token", &mut drafts.token, metrics, theme, true);
    let mut save = false;
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text("machine.llm"),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(&mut drafts.llm).desired_width(field_width.max(40.0)));
        },
        |ui, _| save = qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked(),
    );
    if save {
        drafts.save_ai(root);
    }
    said(ui, &drafts.ai_said, theme);
}

fn readers(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme, drafts: &mut Drafts) {
    title(ui, "machine.readers_title", "machine.readers_hint", metrics, theme);
    let mut removed = None;
    for (index, reader) in drafts.readers.iter().enumerate() {
        qnc_settings_rows::inline_row(
            ui,
            width,
            &reader.name,
            metrics,
            theme,
            |ui, _| {
                ui.label(RichText::new(reader.path.to_string_lossy()).size(metrics.label_font_size).color(theme.text));
            },
            |ui, _| {
                if qnc_settings_rows::action_btn(ui, &text("machine.reader_remove"), theme).clicked() {
                    removed = Some(index);
                }
            },
        );
    }
    if let Some(index) = removed {
        let mut readers = drafts.readers.clone();
        readers.remove(index);
        drafts.write_readers(root, readers);
    }
    field(ui, width, "machine.reader_name", &mut drafts.name, metrics, theme, false);
    let mut add = false;
    let can_add = !drafts.name.trim().is_empty() && !drafts.path.trim().is_empty();
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text("machine.reader_path"),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(&mut drafts.path).desired_width(field_width.max(40.0)).hint_text(text("machine.reader_path_hint")));
        },
        |ui, _| add = qnc_settings_rows::primary_btn(ui, &text("machine.reader_add"), can_add, theme).clicked(),
    );
    if add {
        let mut readers = drafts.readers.clone();
        readers.push(CardReader { name: drafts.name.trim().into(), path: drafts.path.trim().into() });
        drafts.write_readers(root, readers);
        if matches!(drafts.readers_said, Some(Ok(_))) {
            (drafts.name, drafts.path) = (String::new(), String::new());
        }
    }
    said(ui, &drafts.readers_said, theme);
}
