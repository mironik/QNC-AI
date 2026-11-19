//! The poster of a compound statement (user 2026-11-19: "poster sa slike na IN-u prvog dijela"):
//! the exact picture at the IN of its first part, as a statement's poster is the picture at its
//! IN. Decoded in the background through the decoder catalog (`qnc-shot-stills`), one at a time,
//! kept in memory by clip and frame; nothing is written. A picture that cannot be decoded is not
//! asked again in this run.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;

use qnc_image_assets::RgbaImage;
use qnc_still_locator::StillLocator;

/// A picture: its clip and frame.
type Key = (String, u64);

#[derive(Default)]
pub struct CompoundPosters {
    done: HashMap<Key, Arc<RgbaImage>>,
    failed: HashSet<Key>,
    working: Option<(Key, Receiver<Result<RgbaImage, String>>)>,
}

impl std::fmt::Debug for CompoundPosters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompoundPosters").field("done", &self.done.len()).field("failed", &self.failed.len()).finish()
    }
}

impl CompoundPosters {
    pub fn new() -> Self {
        Self::default()
    }

    /// The poster of the first part `(clip, in_frame)`, once decoded.
    pub fn get(&self, clip: &str, frame: u64) -> Option<&Arc<RgbaImage>> {
        self.done.get(&(clip.to_string(), frame))
    }

    /// Takes what was decoded and starts the next picture `wanted` asks for (the first parts of
    /// the compound statements). True when a poster came.
    pub fn poll(&mut self, locator: Option<&StillLocator>, wanted: &[(String, u64)]) -> bool {
        let mut came = false;
        if let Some((key, done)) = &self.working {
            match done.try_recv() {
                Ok(Ok(picture)) => {
                    self.done.insert(key.clone(), Arc::new(picture));
                    came = true;
                    self.working = None;
                }
                Ok(Err(error)) => {
                    log(&format!("compound-poster {} {}: {error}", key.0, key.1));
                    self.failed.insert(key.clone());
                    self.working = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return false,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.failed.insert(key.clone());
                    self.working = None;
                }
            }
        }
        let (Some(locator), Some(next)) = (locator, wanted.iter().find(|key| !self.done.contains_key(*key) && !self.failed.contains(*key))) else { return came };
        let (send, receive) = channel();
        let (locator, key) = (locator.clone(), next.clone());
        let (clip, frame) = key.clone();
        std::thread::spawn(move || {
            // The decoder gives the image crate's picture; the cards show this QNC's.
            let picture = qnc_shot_stills::decode_still(Some(&locator), &clip, frame).map(|picture| {
                let size = [picture.width() as usize, picture.height() as usize];
                RgbaImage { size, pixels: picture.into_raw(), content_key: content_key(&clip, frame) }
            });
            let _ = send.send(picture);
        });
        self.working = Some((key, receive));
        came
    }
}

fn log(line: &str) {
    if qnc_dev_diagnostics_enabled() {
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line.to_string());
    }
}

fn qnc_dev_diagnostics_enabled() -> bool {
    qnc_dev_diagnostics::player_diagnostics_enabled()
}

/// The cache key of a poster: its clip and frame.
fn content_key(clip: &str, frame: u64) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (clip, frame).hash(&mut hasher);
    hasher.finish()
}
