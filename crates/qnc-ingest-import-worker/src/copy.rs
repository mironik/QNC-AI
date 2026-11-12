use crate::*;
use std::{fs, io::{Read, Write}, path::Path, sync::atomic::{AtomicBool, Ordering}};

/// Large pieces read in order: a card behind USB reads fastest so (measured 2026-11-12 on the card
/// reader: 4 MB 33 MB/s, 16 MB 38 MB/s, 32 MB 39 MB/s, the system copy 40 MB/s).
const CHUNK: usize = 32 * 1024 * 1024;
/// Pieces read ahead while the ones before are written (the card never waits for the disk): 64 MB.
const AHEAD: usize = 2;
/// A card that gives no data this long (a bad place of the card, a reader that hangs) fails
/// the clip with a clear reason; the other clips go on and it is imported again when the card
/// is back (user 2026-11-12: "zapelo je na neispravnom klipu").
const STALL: std::time::Duration = std::time::Duration::from_secs(60);

/// Well inside the store lease, so a slow copy never loses its clip.
const HEARTBEAT_EVERY: std::time::Duration = std::time::Duration::from_secs(20);

pub(crate) fn copy_into(
    opener: &dyn MediaOpener,
    source_uri: &str,
    destination: &Path,
    cancel: &AtomicBool,
    beat: &mut dyn FnMut(),
) -> Result<(), String> {
    // A medium on this computer is read straight from its file (read only, large pieces); the
    // stream of the media transport checks the file at every read, which makes a card behind USB
    // crawl (user 2026-11-12: "3,5 gigabajta mora biti za čas"). It is checked once, at the end.
    // A medium of another computer goes through the transport.
    let local = opener.local_path(source_uri);
    let stamp_of = |path: &Path| fs::metadata(path).and_then(|meta| Ok((meta.len(), meta.modified()?))).ok();
    let before = local.as_deref().and_then(stamp_of);
    let source = match &local {
        Some(path) => file_pieces(path)?,
        None => pieces_of(Box::new(opener.open(source_uri)?)),
    };
    let copied = copy_pieces(opener, source, destination, cancel, beat);
    if copied.is_ok() && local.as_deref().and_then(stamp_of) != before {
        return Err(qnc_ui_text::text("import.source_changed").into());
    }
    copied
}

fn copy_pieces(
    opener: &dyn MediaOpener,
    mut source: NextPiece,
    destination: &Path,
    cancel: &AtomicBool,
    beat: &mut dyn FnMut(),
) -> Result<(), String> {
    let mut file = fs::File::create(destination).map_err(|e| e.to_string())?;
    // The card is read on a thread of its own, ahead of the writes. A read the card never answers
    // leaves that thread behind; the copy itself ends after `STALL`.
    let (send, pieces) = std::sync::mpsc::sync_channel::<Result<Vec<u8>, String>>(AHEAD);
    let stop = std::sync::Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    std::thread::spawn(move || loop {
        if stopped.load(Ordering::Relaxed) {
            return;
        }
        let read = source();
        let end = !matches!(&read, Ok(piece) if !piece.is_empty());
        if send.send(read).is_err() || end {
            return;
        }
    });
    let _stop = StopOnDrop(stop);
    let mut last_beat = std::time::Instant::now();
    loop {
        while opener.paused() {
            if cancel.load(Ordering::Relaxed) {
                return Err(qnc_ui_text::text("import.import_interrupted").into());
            }
            if last_beat.elapsed() >= HEARTBEAT_EVERY {
                beat();
                last_beat = std::time::Instant::now();
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        if cancel.load(Ordering::Relaxed) {
            return Err(qnc_ui_text::text("import.import_interrupted").into());
        }
        let piece = match pieces.recv_timeout(STALL) {
            Ok(piece) => piece?,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => return Err(qnc_ui_text::fill("import.card_stalled", &[("seconds", &STALL.as_secs().to_string())])),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Err(qnc_ui_text::text("import.import_interrupted").into()),
        };
        if piece.is_empty() {
            return Ok(());
        }
        file.write_all(&piece).map_err(|e| e.to_string())?;
        if last_beat.elapsed() >= HEARTBEAT_EVERY {
            beat();
            last_beat = std::time::Instant::now();
        }
    }
}

/// The next piece of a medium: empty at its end.
type NextPiece = Box<dyn FnMut() -> Result<Vec<u8>, String> + Send>;

/// Pieces of a reader, in order.
fn pieces_of(mut source: Box<dyn Read + Send>) -> NextPiece {
    Box::new(move || {
        let mut buffer = vec![0_u8; CHUNK];
        let n = fill(&mut *source, &mut buffer)?;
        buffer.truncate(n);
        Ok(buffer)
    })
}

/// A file of this computer read past the system cache, in order, as the system copy does
/// (robocopy /J): measured 2026-11-12 on the card reader, 40 MB/s against 35 MB/s through the cache.
/// Elsewhere, or when the system refuses it, the file is read as any reader.
fn file_pieces(path: &Path) -> Result<NextPiece, String> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
        const FILE_FLAG_SEQUENTIAL_SCAN: u32 = 0x0800_0000;
        // A read past the cache goes into memory aligned to the sector, a whole number of sectors
        // at a time; a page is a whole number of sectors of every disk.
        const ALIGN: usize = 4096;
        if let Ok(mut file) = fs::OpenOptions::new().read(true).custom_flags(FILE_FLAG_NO_BUFFERING | FILE_FLAG_SEQUENTIAL_SCAN).open(path) {
            let mut memory = vec![0_u8; CHUNK + ALIGN];
            let mut ended = false;
            return Ok(Box::new(move || {
                if ended {
                    return Ok(Vec::new());
                }
                let start = memory.as_ptr().align_offset(ALIGN);
                let buffer = &mut memory[start..start + CHUNK];
                let mut filled = 0;
                // Only the last read of the file is shorter than asked; the one after it reads nothing.
                while filled < CHUNK {
                    match file.read(&mut buffer[filled..]) {
                        Ok(0) => break,
                        Ok(n) => {
                            filled += n;
                            if n % ALIGN != 0 {
                                break;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(error) => return Err(error.to_string()),
                    }
                }
                ended = filled < CHUNK;
                Ok(buffer[..filled].to_vec())
            }));
        }
    }
    Ok(pieces_of(Box::new(fs::File::open(path).map_err(|e| e.to_string())?)))
}
/// Reads until the buffer is full or the medium ends.
fn fill(source: &mut dyn Read, buffer: &mut [u8]) -> Result<usize, String> {
    let mut filled = 0;
    while filled < buffer.len() {
        match source.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    Ok(filled)
}

/// Tells the reading thread to stop when the copy ends early.
struct StopOnDrop(std::sync::Arc<AtomicBool>);

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

/// Executes the action for one clip and returns the URI of the imported media.
pub fn import_clip(
    clip: &StoredClip,
    plan: &IngestWorkPlan,
    project_dir: &Path,
    opener: &dyn MediaOpener,
    cancel: &AtomicBool,
) -> Result<String, String> {
    import_clip_beating(clip, plan, project_dir, opener, cancel, &mut || {})
}

/// Like `import_clip`, and calls `beat` regularly while a long copy runs.
pub fn import_clip_beating(
    clip: &StoredClip,
    plan: &IngestWorkPlan,
    project_dir: &Path,
    opener: &dyn MediaOpener,
    cancel: &AtomicBool,
    beat: &mut dyn FnMut(),
) -> Result<String, String> {
    import_media(clip, plan, project_dir, opener, cancel, beat).map(|imported| imported.media_uri)
}

/// What one import made: the imported media (linked or copied), what it is a copy of,
/// and the optimized copy when the project asks for one.
pub(crate) struct Imported {
    pub(crate) media_uri: String,
    pub(crate) copy_of: Option<ImportedCopy>,
    pub(crate) optimized: Option<MediaRepresentation>,
}

pub(crate) fn import_media(
    clip: &StoredClip,
    plan: &IngestWorkPlan,
    project_dir: &Path,
    opener: &dyn MediaOpener,
    cancel: &AtomicBool,
    beat: &mut dyn FnMut(),
) -> Result<Imported, String> {
    let plain = |media_uri, copy_of| Imported { media_uri, copy_of, optimized: None };
    match action_for(clip, plan)? {
        Action::Link { media_uri } => Ok(plain(media_uri, None)),
        Action::Optimize { with_original } => {
            let original = clip.clip.snapshot.binding.original_uri.clone();
            let optimized = optimize::make(clip, plan, project_dir, opener, cancel)?;
            let (media_uri, copy_of) = if with_original {
                (copy_media(clip, plan, project_dir, opener, cancel, beat, &original, Folder::Original)?, Some(ImportedCopy::Original))
            } else {
                (original, None)
            };
            Ok(Imported { media_uri, copy_of, optimized: Some(optimized) })
        }
        Action::Copy { source_uri, folder } => {
            copy_media(clip, plan, project_dir, opener, cancel, beat, &source_uri, folder)
                .map(|uri| plain(uri, Some(folder.copy())))
        }
    }
}

/// Copies one source medium into the project folder and returns its project URI.
#[allow(clippy::too_many_arguments)]
pub(crate) fn copy_media(
    clip: &StoredClip,
    plan: &IngestWorkPlan,
    project_dir: &Path,
    opener: &dyn MediaOpener,
    cancel: &AtomicBool,
    beat: &mut dyn FnMut(),
    source_uri: &str,
    folder: Folder,
) -> Result<String, String> {
    {
        {
            let name = safe_name(clip.clip.id(), source_uri);
            let directory = project_dir.join(folder.name());
            inside_project(project_dir, &directory.join(&name))?;
            fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
            copy_into(opener, source_uri, &directory.join(&name), cancel, beat)?;
            // The copy is read like media on a card: through the project folder as a source.
            let source = plan
                .settings
                .project_media_source_uri()
                .map_err(|e| e.to_string())?;
            let reference = qnc_source_reader::SourceReference::new(
                &source,
                &format!("{}/{name}", folder.name()),
            )
            .map_err(|e| e.to_string())?;
            Ok(reference.uri())
        }
    }
}

/// Writes go only where the project says: below the project folder. A source (a card) is read
/// only, so nothing is ever written, created or changed there.
/// The one guard of where the project may be written (the poster block's).
pub(crate) use qnc_missing_posters::inside_project;

/// The poster of a clip in the project. The card is read only and is never touched.
/// - The card has a poster: the poster is copied into the project thumbnail location, so the
///   clip keeps its picture when the card is gone, even when the media stays linked.
/// - The card has no poster: it is created from a key frame of the media, the way the
///   filmstrip frames are made, at poster resolution.
/// A poster that cannot be made never fails the import.
pub fn import_poster(
    clip: &StoredClip,
    plan: &IngestWorkPlan,
    project_dir: &Path,
    opener: &dyn MediaOpener,
    cancel: &AtomicBool,
) -> Option<String> {
    let clip_id = clip.clip.id();
    let output = qnc_missing_posters::poster_file(&plan.settings, project_dir, clip_id).ok()?;
    let directory = output.parent()?.to_path_buf();
    let made = format!("{}/{clip_id}/poster.jpg", plan.thumbnails_uri.trim_end_matches('/'));
    match clip.clip.thumbnail_uri.as_deref() {
        // Already made in the project (a single file without a card poster): kept.
        Some(uri) if uri == made => {}
        Some(source_uri) => {
            fs::create_dir_all(&directory).ok()?;
            copy_into(opener, source_uri, &output, cancel, &mut || {}).ok()?;
        }
        // The start of the clip, by the one block that makes missing posters.
        None => qnc_missing_posters::poster_from_start(clip, &plan.settings, &|uri: &str| opener.local_path(uri), &output, cancel).ok()?,
    }
    Some(made)
}

/// Copies a file of this computer as an import copies a medium of a card (diagnostic: the copy
/// speed against the system copy).
pub fn copy_local_file(source: &Path, destination: &Path) -> Result<(), String> {
    struct Local(std::path::PathBuf);
    impl MediaOpener for Local {
        fn open(&self, _: &str) -> Result<Box<dyn MediaRead>, String> {
            Err("read from the file".into())
        }
        fn local_path(&self, _: &str) -> Option<std::path::PathBuf> {
            Some(self.0.clone())
        }
    }
    copy_into(&Local(source.to_path_buf()), "", destination, &AtomicBool::new(false), &mut || {})
}
