//! The tray's look at Incoming (user 2026-11-24: "daj opcije u postavci inkoming: automatski
//! ingest ili rucni ingest"). With automatic ingest set, the tray looks at Incoming's folder now
//! and then; when the files in it are other than those it ingested last and have stopped changing
//! (a file sent over FTP or Filemail grows until it is all there), it says Incoming is ready, and
//! the tray ingests it as a folder (Select keeps the clips it has and adds the new ones). The folder
//! is only read: names, sizes and last writes. What was ingested is remembered while the tray runs;
//! after a start the first look ingests once more, which takes nothing twice.

use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

/// How often the folder is looked at.
pub const LOOK: Duration = Duration::from_secs(5);
/// How long what arrived must stay as it is before it is ingested.
pub const STILL: Duration = Duration::from_secs(10);
/// How deep the folder is looked into (as Select of a folder).
const DEPTH: usize = 8;

/// What the folder holds: every file (its path, size and last write), in order.
type Contents = Vec<(String, u64, Option<SystemTime>)>;

/// The tray's look at Incoming.
#[derive(Default)]
pub struct IncomingWatch {
    looked: Option<Instant>,
    /// The look running on its own thread (a server's folder may answer slowly).
    looking: Option<std::sync::mpsc::Receiver<Result<Option<(qnc_incoming::Incoming, Contents)>, String>>>,
    /// What was seen last and since when it is so.
    seen: Option<(Contents, Instant)>,
    /// What was handed to an ingest last.
    taken: Option<Contents>,
}

impl IncomingWatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks at Incoming of `root` when it is time: its source (to ingest as a folder) when it is
    /// set to automatic ingest and holds something new that stopped changing; otherwise none.
    pub fn look(&mut self, root: &Path) -> Option<Result<String, String>> {
        if let Some(looking) = &self.looking {
            let answer = match looking.try_recv() {
                Ok(answer) => answer,
                Err(std::sync::mpsc::TryRecvError::Empty) => return None,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => Ok(None),
            };
            self.looking = None;
            return match answer {
                Ok(Some((incoming, contents))) => self.ready(contents, Instant::now()).map(|()| incoming.source_uri()),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            };
        }
        if self.looked.is_some_and(|at| at.elapsed() < LOOK) {
            return None;
        }
        self.looked = Some(Instant::now());
        let (send, receive) = std::sync::mpsc::channel();
        let root = root.to_path_buf();
        let started = std::thread::Builder::new().name("qnc-incoming-watch".into()).spawn(move || {
            let answer = match qnc_incoming::read(&root) {
                Ok(Some(incoming)) if incoming.ingest == qnc_incoming::Ingest::Auto => {
                    let found = contents(&incoming.folder());
                    Ok(Some((incoming, found)))
                }
                Ok(_) => Ok(None),
                Err(error) => Err(error),
            };
            let _ = send.send(answer);
        });
        if started.is_ok() {
            self.looking = Some(receive);
        }
        None
    }

    /// What was taken last is let go (its ingest could not start): the next look takes it again.
    pub fn again(&mut self) {
        self.taken = None;
    }

    /// Whether `contents` seen `now` is to be ingested: something, other than what was taken last,
    /// the same for `STILL`.
    fn ready(&mut self, contents: Contents, now: Instant) -> Option<()> {
        match &self.seen {
            Some((seen, since)) if *seen == contents => {
                let new = !contents.is_empty() && self.taken.as_ref() != Some(&contents);
                if new && now.duration_since(*since) >= STILL {
                    self.taken = Some(contents);
                    return Some(());
                }
                None
            }
            _ => {
                self.seen = Some((contents, now));
                None
            }
        }
    }
}

/// The files under `folder` (a folder that cannot be read holds nothing now); hidden files and
/// those still being written by a sender (`.part`, `.tmp`, `.filepart`, `.crdownload`) are left out.
fn contents(folder: &Path) -> Contents {
    let mut found = Vec::new();
    walk(folder, folder, 0, &mut found);
    found.sort();
    found
}

fn walk(root: &Path, folder: &Path, depth: usize, found: &mut Contents) {
    let Ok(entries) = std::fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else { continue };
        let path = entry.path();
        if kind.is_dir() && depth < DEPTH {
            walk(root, &path, depth + 1, found);
        } else if kind.is_file() {
            let lower = name.to_ascii_lowercase();
            if [".part", ".tmp", ".filepart", ".crdownload"].iter().any(|end| lower.ends_with(end)) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy().into_owned();
            found.push((relative, meta.len(), meta.modified().ok()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_arrived_is_ingested_once_it_stops_changing_and_only_once() {
        let folder = tempfile::tempdir().unwrap();
        let mut watch = IncomingWatch::new();
        let start = Instant::now();
        assert!(watch.ready(contents(folder.path()), start).is_none(), "nothing arrived");
        std::fs::write(folder.path().join("A.MXF"), b"part").unwrap();
        std::fs::write(folder.path().join("B.mp4.part"), b"still coming").unwrap();
        let first = contents(folder.path());
        assert_eq!(first.len(), 1, "a file still being sent is left out");
        assert!(watch.ready(first.clone(), start).is_none(), "just seen");
        assert!(watch.ready(first.clone(), start + Duration::from_secs(3)).is_none(), "still changing for all we know");
        assert!(watch.ready(first.clone(), start + STILL).is_some(), "it stopped changing");
        assert!(watch.ready(first, start + STILL * 3).is_none(), "taken: not twice");
        std::fs::write(folder.path().join("A.MXF"), b"part and more").unwrap();
        let grown = contents(folder.path());
        let later = start + STILL * 4;
        assert!(watch.ready(grown.clone(), later).is_none());
        assert!(watch.ready(grown, later + STILL).is_some(), "it changed: ingested again");
    }
}
