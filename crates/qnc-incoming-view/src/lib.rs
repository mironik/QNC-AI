//! A block of QNC-AI's Settings form: Incoming of this computer (user 2026-11-24: "inkoming treba
//! dodati u postavkama i u njima postavke putanje direktorija"; "moze biti i na serveru, a moze biti
//! i u racunalu"; "automatski ingest ili rucni ingest"). Where it is (this computer, LAN, intranet),
//! its folder or address, and how it is ingested; Save keeps it (`qnc-incoming`), Remove takes
//! Incoming away. Nothing is chosen for the user.

use std::path::Path;

use eframe::egui;
use qnc_incoming::{Incoming, Ingest, Place};
use qnc_settings_block::Pen;
use qnc_wizard_text::text;

#[derive(Clone)]
struct Draft {
    place: String,
    location: String,
    ingest: String,
    said: Option<Result<String, String>>,
}

impl Draft {
    fn read(root: &Path) -> Self {
        match qnc_incoming::read(root) {
            Ok(Some(incoming)) => Self { place: incoming.place.code().into(), location: incoming.location, ingest: incoming.ingest.code().into(), said: None },
            Ok(None) => Self { place: String::new(), location: String::new(), ingest: String::new(), said: None },
            Err(error) => Self { place: String::new(), location: String::new(), ingest: String::new(), said: Some(Err(error)) },
        }
    }

    /// What is typed as an Incoming, or why it is none.
    fn incoming(&self) -> Result<Incoming, String> {
        let place = Place::of(&self.place).ok_or_else(|| text("incoming.choose_place").to_string())?;
        let ingest = Ingest::of(&self.ingest).ok_or_else(|| text("incoming.choose_ingest").to_string())?;
        let incoming = Incoming { place, location: self.location.trim().to_string(), ingest };
        incoming.check()?;
        Ok(incoming)
    }
}

/// Draws the block with the settings style block's pen.
pub fn show(ui: &mut egui::Ui, root: &Path, pen: &Pen) {
    let id = egui::Id::new(("qnc-incoming-view", root.to_string_lossy().to_string()));
    let mut draft = ui.ctx().data_mut(|data| data.get_temp::<Draft>(id)).unwrap_or_else(|| Draft::read(root));
    pen.title(ui, "incoming.title", "incoming.hint");
    let places: Vec<(String, String)> = Place::ALL.iter().map(|place| (place.code().to_string(), text(place.label()).to_string())).collect();
    if let Some(place) = pen.choice(ui, &text("incoming.place"), "qnc-incoming-place", &draft.place, &places) {
        draft.place = place;
    }
    let hint = match Place::of(&draft.place) {
        Some(Place::Computer) => text("incoming.location_computer"),
        Some(_) => text("incoming.location_network"),
        None => text("incoming.location_hint"),
    };
    pen.text(ui, "incoming.location", &mut draft.location, &hint);
    let ways: Vec<(String, String)> = Ingest::ALL.iter().map(|ingest| (ingest.code().to_string(), text(ingest.label()).to_string())).collect();
    if let Some(ingest) = pen.choice(ui, &text("incoming.ingest"), "qnc-incoming-ingest", &draft.ingest, &ways) {
        draft.ingest = ingest;
    }
    let (mut save, mut remove) = (false, false);
    pen.buttons(ui, |ui, buttons| {
        save = buttons.primary(ui, &text("machine.save"), true);
        remove = buttons.action(ui, &text("incoming.remove"));
    });
    if save {
        draft.said = Some(draft.incoming().and_then(|incoming| qnc_incoming::write(root, Some(&incoming))).map(|()| text("machine.saved").to_string()));
    }
    if remove {
        draft.said = Some(qnc_incoming::write(root, None).map(|()| text("incoming.removed").to_string()));
        if draft.said.as_ref().is_some_and(Result::is_ok) {
            (draft.place, draft.location, draft.ingest) = (String::new(), String::new(), String::new());
        }
    }
    pen.said(ui, &draft.said);
    ui.ctx().data_mut(|data| data.insert_temp(id, draft));
}
