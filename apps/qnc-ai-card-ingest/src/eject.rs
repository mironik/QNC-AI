//! The card taken out by itself (user 2026-11-18: "automatsko izbacivanje kartice nakon odustani i
//! nakon što je uvoz završio posao"): after a confirmed Cancel, and once the import has copied
//! every clip into the base when the base keeps nothing on the card (its media copied whole: the
//! original, or the optimized copy with the original). A card the base still reads (linked media,
//! a copied proxy whose sound is the original's) stays in. The eject is `qnc-card-eject`'s, never
//! by force: a disk still in use is tried again for a while, then said.

use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// How long a disk still in use (the work stopping after a Cancel) is tried again.
const TRIES: u32 = 40;
const AGAIN: Duration = Duration::from_millis(500);

pub(crate) struct CardOut {
    /// The card's volume serial (the window's `--card`); none for a folder.
    serial: Option<String>,
    /// The base keeps nothing on the card once its import is done.
    free_when_imported: bool,
    started: bool,
    /// Out after the import: the window closes once the card is out (user 2026-11-18:
    /// "automatsko zatvaranje ingest vizarda kad je kartica ejektirana"; the work goes on in the
    /// background).
    closes_window: bool,
    answer: Option<Receiver<Result<(), String>>>,
    said: Option<Result<(), String>>,
}

impl CardOut {
    pub(crate) fn new(serial: Option<String>, free_when_imported: bool) -> Self {
        Self { serial, free_when_imported, started: false, closes_window: false, answer: None, said: None }
    }

    /// The import has copied `imported` of `clips` clips, `failed` failed: the card goes out once
    /// all are copied and the base needs it no more.
    pub(crate) fn imported(&mut self, clips: usize, imported: usize, failed: usize) {
        if self.free_when_imported && clips > 0 && imported >= clips && failed == 0 && !self.started {
            self.closes_window = true;
            self.start("import done");
        }
    }

    /// The card is out after the import: the window closes (said once; a window still waiting for
    /// its base's name asks for it as at any close).
    pub(crate) fn window_closes(&mut self) -> bool {
        let closes = self.closes_window && matches!(self.said, Some(Ok(())));
        if closes {
            self.closes_window = false;
        }
        closes
    }

    /// The card goes out (once).
    pub(crate) fn start(&mut self, why: &str) {
        let Some(serial) = self.serial.clone().filter(|_| !self.started) else { return };
        self.started = true;
        crate::log(format!("ai-card-ingest card {serial} ejected after: {why}"));
        let (send, answer) = channel();
        self.answer = Some(answer);
        std::thread::spawn(move || {
            let mut last = String::new();
            for _ in 0..TRIES {
                let Some(volume) = qnc_source_volumes::mounted().into_iter().find(|volume| volume.serial_number == serial) else {
                    // Out already (taken out by hand).
                    let _ = send.send(Ok(()));
                    return;
                };
                match qnc_card_eject::eject(&volume.root) {
                    Ok(()) => {
                        let _ = send.send(Ok(()));
                        return;
                    }
                    Err(qnc_card_eject::EjectError::InUse) => last = qnc_card_eject::EjectError::InUse.to_string(),
                    Err(error) => {
                        let _ = send.send(Err(error.to_string()));
                        return;
                    }
                }
                std::thread::sleep(AGAIN);
            }
            let _ = send.send(Err(last));
        });
    }

    /// What became of the eject, once known: the line the window says.
    pub(crate) fn said(&mut self) -> Option<String> {
        if let Some(result) = self.answer.as_ref().and_then(|answer| answer.try_recv().ok()) {
            crate::log(format!("ai-card-ingest card eject: {result:?}"));
            self.said = Some(result);
            self.answer = None;
        }
        self.said.as_ref().map(|result| match result {
            Ok(()) => qnc_wizard_text::text("card_ingest.card_ejected").to_string(),
            Err(error) => qnc_wizard_text::fill("card_ingest.card_not_ejected", &[("error", error)]),
        })
    }
}

/// Whether the base `base` keeps nothing on the card once its import is done, by its settings.
pub(crate) fn free_when_imported(root: &std::path::Path, base: &str) -> bool {
    let media = qnc_work_settings::SettingsReader::from_root(root)
        .map_err(|error| error.to_string())
        .and_then(|reader| reader.with_project(base).read().map_err(|error| error.to_string()))
        .and_then(qnc_ingest_work_plan::IngestWorkPlan::from_settings)
        .map(|plan| plan.media);
    match media {
        Ok(media) => matches!(media, qnc_ingest_work_plan::IngestMedia::Original | qnc_ingest_work_plan::IngestMedia::Optimized { with_original: true }),
        Err(error) => {
            crate::log(format!("ai-card-ingest the card stays in: settings not read: {error}"));
            false
        }
    }
}
