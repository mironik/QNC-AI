//! The cards of known cameras attached now (user 2026-11-11: "ingest cim se ucita skenira karticu,
//! ne ceka korisnika; ako se ubaci nova kartica i ona se skenira"). A thread of its own looks at
//! the attached disks every two seconds with the camera catalog of the root and gives the volume
//! serials of those that are cards; the caller takes them when it likes. It reads only.

use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// How often the disks are looked at.
const LOOK: Duration = Duration::from_secs(2);

/// The attached cards, looked at in the background.
pub struct AttachedCards {
    found: Receiver<Result<Vec<String>, String>>,
}

impl AttachedCards {
    /// Starts looking at the disks with the camera catalog of `root`.
    pub fn start(root: &Path) -> Self {
        let (send, found) = channel();
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            let mut catalog = None;
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
                    let cards = qnc_source_volumes::mounted()
                        .into_iter()
                        .filter(|volume| matches!(qnc_card_watch::card_patterns(catalog, &volume.root, &volume.source_uri()), Ok(patterns) if !patterns.is_empty()))
                        .map(|volume| volume.serial_number)
                        .collect();
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
    pub fn poll(&mut self) -> Option<Result<Vec<String>, String>> {
        let mut newest = None;
        while let Ok(found) = self.found.try_recv() {
            newest = Some(found);
        }
        newest
    }
}
