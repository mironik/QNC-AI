//! The clips of the cards a story has, before it has a project (user 2026-11-11: "modificirani
//! Ingest"; "kod mora biti odvojen od GUI"). It reads the temporary JSON of every card read by the
//! tray (`qnc-card-read`) again every second, loads each clip's poster from its card (read only),
//! and keeps which shots are ticked for import. It draws nothing: a form shows `rows()` and sends
//! `toggle`, `tick_all`, `tick_none` back. Ticks live while the program runs; they become the
//! project's when the story is created (W3).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

use qnc_card_read::CardContent;
use qnc_clip_row::{ClipRow, ThumbStatus};
use qnc_image_assets::RgbaImage;
use qnc_source_reader::{SourceReader, SourceReference};

/// How often the temporary files are read again.
const READ_EVERY: Duration = Duration::from_secs(1);

/// The id of a clip in the pick: its card and its name.
pub fn clip_id(serial: &str, name: &str) -> String {
    format!("{serial}:{name}")
}

/// The pick of one story.
pub struct CardPick {
    cards: Vec<CardContent>,
    rows: Vec<ClipRow>,
    ticked: BTreeSet<String>,
    posters: BTreeMap<String, Arc<RgbaImage>>,
    asked: BTreeSet<String>,
    read: Receiver<Vec<CardContent>>,
    ask: Sender<(String, String, PathBuf)>,
    loaded: Receiver<(String, Option<Arc<RgbaImage>>)>,
}

impl CardPick {
    /// Starts reading the cards of the QNC-AI root `root` in the background.
    pub fn new(root: &Path) -> Self {
        let (send, read) = channel();
        let folder = root.to_path_buf();
        std::thread::spawn(move || loop {
            if send.send(qnc_card_read::all_kept(&folder)).is_err() {
                return;
            }
            std::thread::sleep(READ_EVERY);
        });
        let (ask, asked_for) = channel::<(String, String, PathBuf)>();
        let (answer, loaded) = channel();
        std::thread::spawn(move || {
            for (uri, source_uri, disk) in asked_for {
                let poster = poster(&uri, &source_uri, &disk).ok().map(Arc::new);
                if answer.send((uri, poster)).is_err() {
                    return;
                }
            }
        });
        Self { cards: Vec::new(), rows: Vec::new(), ticked: BTreeSet::new(), posters: BTreeMap::new(), asked: BTreeSet::new(), read, ask, loaded }
    }

    /// Takes what the background read; true when what is shown changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(mut cards) = self.read.try_recv() {
            cards.sort_by(|a, b| a.serial.cmp(&b.serial));
            if cards != self.cards {
                self.cards = cards;
                changed = true;
            }
        }
        while let Ok((uri, poster)) = self.loaded.try_recv() {
            if let Some(poster) = poster {
                self.posters.insert(uri, poster);
                changed = true;
            }
        }
        if changed {
            self.lay_out();
        }
        changed
    }

    fn lay_out(&mut self) {
        let mut rows = Vec::new();
        for card in &self.cards {
            for clip in &card.clips {
                let id = clip_id(&card.serial, &clip.name);
                if let Some(uri) = clip.thumbnail.as_ref().filter(|uri| !self.asked.contains(*uri)) {
                    self.asked.insert(uri.clone());
                    let _ = self.ask.send((uri.clone(), card.source_uri.clone(), PathBuf::from(&card.disk)));
                }
                let poster: Option<Arc<RgbaImage>> = clip.thumbnail.as_ref().and_then(|uri| self.posters.get(uri).cloned());
                rows.push(ClipRow {
                    selected: self.ticked.contains(&id),
                    clip_id: id,
                    name: clip.name.clone(),
                    duration_seconds: clip.duration_seconds.unwrap_or(0.0),
                    thumb_uri: clip.thumbnail.clone(),
                    thumb_status: if poster.is_some() { ThumbStatus::Ready } else { ThumbStatus::Pending },
                    thumb_image: poster,
                    ..Default::default()
                });
            }
        }
        self.rows = rows;
    }

    /// The clips of every card, a card after a card.
    pub fn rows(&self) -> &[ClipRow] {
        &self.rows
    }

    /// The cards read.
    pub fn cards(&self) -> &[CardContent] {
        &self.cards
    }

    /// Ticks or unticks a clip.
    pub fn toggle(&mut self, id: &str) {
        if !self.ticked.remove(id) {
            self.ticked.insert(id.to_string());
        }
        self.lay_out();
    }

    pub fn tick_all(&mut self) {
        self.ticked = self.rows.iter().map(|row| row.clip_id.clone()).collect();
        self.lay_out();
    }

    pub fn tick_none(&mut self) {
        self.ticked.clear();
        self.lay_out();
    }

    /// How many clips are ticked.
    pub fn ticked(&self) -> usize {
        self.rows.iter().filter(|row| row.selected).count()
    }
}

/// The poster `uri` of a card, read only from its disk.
fn poster(uri: &str, source_uri: &str, disk: &Path) -> Result<RgbaImage, String> {
    let source = SourceReader::local(source_uri, disk).map_err(|error| format!("{error:?}"))?;
    let reference = SourceReference::from_uri(uri).map_err(|error| format!("{error:?}"))?;
    let bytes = source.read_bytes(&reference, qnc_image_assets::MAX_BYTES as u64).map_err(|error| format!("{error:?}"))?;
    qnc_image_assets::decode_thumbnail(&bytes.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qnc_card_read::CardClip;

    #[test]
    fn the_clips_of_the_cards_are_shown_and_ticked() {
        let root = std::env::temp_dir().join(format!("qnc-card-pick-{}", std::process::id()));
        let clip = |name: &str| CardClip { name: name.into(), original: "o".into(), proxies: vec![], thumbnail: None, duration_seconds: Some(3.0) };
        let card = CardContent { serial: "s1".into(), disk: "X:\\".into(), source_uri: "u".into(), read_at_unix_ms: 1, clips: vec![clip("A"), clip("B")], warnings: vec![] };
        qnc_card_read::keep(&root, &card).unwrap();
        let mut pick = CardPick::new(&root);
        for _ in 0..200 {
            if pick.poll() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(pick.rows().len(), 2);
        pick.toggle(&clip_id("s1", "B"));
        assert_eq!(pick.ticked(), 1);
        assert!(pick.rows()[1].selected);
        pick.tick_all();
        assert_eq!(pick.ticked(), 2);
        pick.tick_none();
        assert_eq!(pick.ticked(), 0);
        let _ = std::fs::remove_dir_all(&root);
    }
}
