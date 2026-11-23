//! A block of QNC-AI's Settings form (user 2026-11-22: "putanju importa kroz čitač kartica spojen na
//! udaljenoj lokaciji"; 2026-11-23: "forma postavke se slaže od kocaka"): the card readers on the
//! network of this computer (`qnc-card-readers`), each a name and the shared folder, written at
//! once on Add and Remove; Ingest shows each under LAN.

use std::path::Path;

use eframe::egui;
use qnc_card_readers::CardReader;
use qnc_settings_block::Pen;
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

/// Draws the block with the settings style block's pen.
pub fn show(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let id = egui::Id::new(("qnc-card-readers-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    pen.title(ui, "machine.readers_title", "machine.readers_hint");
    let mut removed = None;
    for (index, reader) in draft.readers.iter().enumerate() {
        pen.row(ui, &reader.name, |ui, _| pen.hint(ui, &reader.path.to_string_lossy()), |ui, buttons| {
            if buttons.action(ui, &text("machine.reader_remove")) {
                removed = Some(index);
            }
        });
    }
    if let Some(index) = removed {
        let mut readers = draft.readers.clone();
        readers.remove(index);
        draft.write(root, readers);
    }
    pen.text(ui, "machine.reader_name", &mut draft.name, "");
    let mut add = false;
    let can_add = !draft.name.trim().is_empty() && !draft.path.trim().is_empty();
    pen.row(
        ui,
        &text("machine.reader_path"),
        |ui, row| {
            row.input(ui, &mut draft.path, row.width, &text("machine.reader_path_hint"));
        },
        |ui, buttons| add = buttons.primary(ui, &text("machine.reader_add"), can_add),
    );
    if add {
        let mut readers = draft.readers.clone();
        readers.push(CardReader { name: draft.name.trim().into(), path: draft.path.trim().into() });
        draft.write(root, readers);
        if matches!(draft.said, Some(Ok(_))) {
            (draft.name, draft.path) = (String::new(), String::new());
        }
    }
    pen.said(ui, &draft.said);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}
