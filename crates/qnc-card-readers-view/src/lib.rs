//! A block of QNC-AI's Settings form (user 2026-11-22: "putanju importa kroz čitač kartica spojen na
//! udaljenoj lokaciji"; 2026-11-23: "forma postavke se slaže od kocaka"): the card readers on the
//! network of this computer (`qnc-card-readers`), each a name and the shared folder, written at
//! once on Add and Remove; Ingest shows each under LAN.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_card_readers::CardReader;
use qnc_desktop_theme::Theme;
use qnc_settings_block::{field, said, title};
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;

#[derive(Clone, Default)]
struct Draft {
    readers: Vec<CardReader>,
    name: String,
    path: String,
    said: Option<Result<String, String>>,
}

impl Draft {
    fn read(root: &Path) -> Self {
        match qnc_card_readers::readers_of(root) {
            Ok(readers) => Self { readers, ..Self::default() },
            Err(error) => Self { said: Some(Err(error)), ..Self::default() },
        }
    }

    fn write(&mut self, root: &Path, readers: Vec<CardReader>) {
        match qnc_card_readers::set_readers_of(root, &readers) {
            Ok(()) => {
                self.readers = readers;
                self.said = Some(Ok(text("machine.saved")));
            }
            Err(error) => self.said = Some(Err(error)),
        }
    }
}

/// Draws the block `width` wide, in the rows of the settings panel.
pub fn show(ui: &mut egui::Ui, root: &Path, width: f32, metrics: &PanelMetrics, theme: &Theme) {
    let id = egui::Id::new(("qnc-card-readers-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    title(ui, "machine.readers_title", "machine.readers_hint", metrics, theme);
    let mut removed = None;
    for (index, reader) in draft.readers.iter().enumerate() {
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
        let mut readers = draft.readers.clone();
        readers.remove(index);
        draft.write(root, readers);
    }
    field(ui, width, "machine.reader_name", &mut draft.name, metrics, theme, false);
    let mut add = false;
    let can_add = !draft.name.trim().is_empty() && !draft.path.trim().is_empty();
    qnc_settings_rows::inline_row(
        ui,
        width,
        &text("machine.reader_path"),
        metrics,
        theme,
        |ui, field_width| {
            ui.add(qnc_ui_kit::text_field(&mut draft.path).desired_width(field_width.max(40.0)).hint_text(text("machine.reader_path_hint")));
        },
        |ui, _| add = qnc_settings_rows::primary_btn(ui, &text("machine.reader_add"), can_add, theme).clicked(),
    );
    if add {
        let mut readers = draft.readers.clone();
        readers.push(CardReader { name: draft.name.trim().into(), path: draft.path.trim().into() });
        draft.write(root, readers);
        if matches!(draft.said, Some(Ok(_))) {
            (draft.name, draft.path) = (String::new(), String::new());
        }
    }
    said(ui, &draft.said, theme);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}
