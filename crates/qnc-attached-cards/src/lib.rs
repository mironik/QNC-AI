//! The cards of known cameras attached now (user 2026-11-11: "ingest cim se ucita skenira karticu,
//! ne ceka korisnika; ako se ubaci nova kartica i ona se skenira"). A thread of its own looks at
//! the attached disks every two seconds with the camera catalog of the root and gives those that
//! are cards, each with its camera (user 2026-11-12: "nazovi karticu po kameri", told once per card
//! from its records by the camera profiles); the caller takes them when it likes. It reads only.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// How often the disks are looked at.
const LOOK: Duration = Duration::from_secs(2);

/// A card attached now: its volume serial and the camera that wrote it, when it is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedCard {
    pub serial: String,
    pub camera: Option<String>,
}

/// The attached cards, looked at in the background.
pub struct AttachedCards {
    found: Receiver<Result<Vec<AttachedCard>, String>>,
}

impl AttachedCards {
    /// Starts looking at the disks with the camera catalog and the camera profiles of `root`.
    pub fn start(root: &Path) -> Self {
        let (send, found) = channel();
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            let mut catalog = None;
            let (profiles, _) = qnc_camera_profiles::read_profiles(&root.join(qnc_camera_profiles::PROFILE_DIR));
            // The camera of each card seen, by serial: its records are read once.
            let mut cameras: BTreeMap<String, Option<String>> = BTreeMap::new();
            loop {
                if catalog.is_none() {
                    match qnc_card_watch::catalog(&root) {
                        Ok(read) => catalog = Some(read),
                        Err(error) => {
                            if send.send(Err(error)).is_err() {
                                return;
                            }
                        }
                    }
                }
                if let Some(catalog) = &catalog {
                    let cards: Vec<AttachedCard> = qnc_source_volumes::mounted()
                        .into_iter()
                        .filter(|volume| matches!(qnc_card_watch::card_patterns(catalog, &volume.root, &volume.source_uri()), Ok(patterns) if !patterns.is_empty()))
                        .map(|volume| {
                            let camera = cameras.entry(volume.serial_number.clone()).or_insert_with(|| qnc_card_watch::camera_of(catalog, &profiles, &volume)).clone();
                            AttachedCard { serial: volume.serial_number, camera }
                        })
                        .collect();
                    cameras.retain(|serial, _| cards.iter().any(|card| &card.serial == serial));
                    if send.send(Ok(cards)).is_err() {
                        return;
                    }
                }
                std::thread::sleep(LOOK);
            }
        });
        Self { found }
    }

    /// The cards of the newest look since the last call, if a look came.
    pub fn poll(&mut self) -> Option<Result<Vec<AttachedCard>, String>> {
        let mut newest = None;
        while let Ok(found) = self.found.try_recv() {
            newest = Some(found);
        }
        newest
    }
}
