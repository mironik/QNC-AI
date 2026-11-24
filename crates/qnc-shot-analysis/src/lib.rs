//! Shot analysis of the clips of a project (user 2026-10-27: Video AI in AI Media Assist; a
//! background job of its own after the filmstrip, lower than the player, on by the project's
//! setting `ai.enabled` "AI shot analysis and SubClips"). The background worker runs this block
//! after the filmstrips and waves; it knows no form.
//!
//! For each imported clip not yet analysed: one intra picture of its original about every
//! second, read through the original's own MXF index (the card is only read, nothing else of
//! the file), decoded small and grey through the decoder catalog, the shots found by
//! `qnc-shot-detect`, kept in the project database by `qnc-shot-db` under the caller's lease.
//! The clip a preview has open goes first; while the player plays it waits. An original that
//! is not on this computer is left for a later run; one without a usable index (not MXF, a
//! format not read yet) is recorded with that reason.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use qnc_content_store::{Access, ContentTarget};
use qnc_db_broker::{ProjectDbTarget, WriteLease};
use qnc_shot_db::{ClipShots, PartRecord, PictureStrip, ShotRecord, StripPicture};
use qnc_work_settings::{SettingsReader, WorkSettings};

mod keys;
pub use keys::KeyPictures;

pub const MODULE_ID: &str = "qnc.module.shot-analysis";

/// Where a media URI is on this computer; `None` when it is not here.
pub type LocalMedia<'a> = dyn Fn(&str) -> Option<PathBuf> + Sync + 'a;

/// The pictures a shot analysis looks at: their size and at most how many a second (two: a
/// camera's long GOP gives about two intra pictures a second; user 2026-11-18 "spusti na 2", an
/// all-intra original was looked at four times a second for nothing).
const PICTURE: [u32; 2] = qnc_filmstrip::handed::THUMB_SIZE;
const MOST_A_SECOND: f64 = 2.0;
/// Pictures decoded in one decoder call.
const BATCH: usize = 16;
/// Decoder calls at once at most (user 2026-11-14: the AI shots on several cores; measured on the
/// laptop: 4 at once 2.1x faster than one); never more than half the threads of this computer.
const MOST_DECODERS: usize = 4;

/// How many decoder calls run at once on this computer.
fn decoders() -> usize {
    let threads = std::thread::available_parallelism().map_or(2, usize::from);
    (threads / 2).clamp(1, MOST_DECODERS)
}

/// The project whose clips are analysed: its settings as read from its database.
#[derive(Debug, Clone)]
pub struct ShotProject {
    reader: SettingsReader,
    settings: WorkSettings,
}

impl ShotProject {
    pub fn new(reader: SettingsReader, settings: WorkSettings) -> Self {
        Self { reader, settings }
    }

    /// The active project of the QNC root, read from its database now.
    pub fn active(root: &Path) -> Result<Self, String> {
        let active = qnc_active_project_read::ActiveProjectReader::from_root(root).map_err(|e| e.to_string())?;
        let snapshot = active.read().map_err(|e| e.to_string())?;
        Ok(Self::new(active.settings_reader().clone(), snapshot.settings))
    }
}

/// What the caller says while the analysis runs.
pub struct Pace<'a> {
    /// The player plays now: the analysis waits.
    pub player_plays: &'a (dyn Fn() -> bool + Sync),
    /// The clip a preview has open, analysed first.
    pub wanted: &'a (dyn Fn() -> Option<String> + Sync),
    /// Which clips this turn may look at (user 2026-11-12: a clip whose first channel holds a
    /// statement is not analysed); `&|_| true` for every clip.
    pub takes: &'a (dyn Fn(&str) -> bool + Sync),
    /// How many clips may be analysed at once now (user 2026-11-19: "kad završi prijepis AI
    /// kadrovima daš maksimalnu raspoloživu snagu"); none while the transcription runs ("dok traje
    /// prijepis ne daj mu ništa"): the analysis then waits.
    pub workers: &'a (dyn Fn() -> usize + Sync),
    /// The run stops (the project closed, its lease lost).
    pub cancel: &'a AtomicBool,
}

/// How many clips at most are analysed at once on this computer: as many as half its threads.
pub fn most_workers() -> usize {
    (std::thread::available_parallelism().map_or(2, usize::from) / 2).max(1)
}

/// Analyses every imported clip of the project not analysed yet; returns how many it kept.
/// Nothing is done when the project has shot analysis off. As many clips at once as
/// `pace.workers` says (asked again before every clip): most clips are short, one batch of
/// pictures, so one clip keeps one decoder busy (2026-11-19 measurement: 58 ms a picture).
pub fn analyse_missing(project: &ShotProject, local_media: &LocalMedia<'_>, lease: Option<WriteLease>, pace: &Pace<'_>) -> Result<usize, String> {
    let (reader, settings) = (&project.reader, &project.settings);
    if !settings.ai_feature(qnc_ai_features::SHOT_ANALYSIS).unwrap_or(false) {
        return Ok(0);
    }
    let shots_db = ProjectDbTarget::for_project(reader, settings)?;
    let analysed = qnc_shot_db::read_analysed(&shots_db)?;
    let catalog = qnc_content_read::ContentReader::for_project(reader, settings)?;
    let waiting = catalog.records_waiting()?;
    let remaining: Vec<String> = catalog
        .summaries()?
        .into_iter()
        .filter(|clip| clip.imported && !analysed.contains(&clip.clip_id) && !waiting.contains(&clip.clip_id) && (pace.takes)(&clip.clip_id))
        .map(|clip| clip.clip_id)
        .collect();
    if remaining.is_empty() {
        return Ok(0);
    }
    // Where every picture the analysis decodes is kept: the folder of the AI shots, a folder a clip
    // (user 2026-11-18: "koliko bi nas koštalo da sve slike spremamo"); without it the analysis
    // goes on and keeps none.
    let strips = strip_folder(reader, settings);
    let content = ContentTarget::for_project(reader, settings)?;
    let remaining = std::sync::Mutex::new(remaining);
    let kept = std::sync::atomic::AtomicUsize::new(0);
    let failure = std::sync::Mutex::new(None::<String>);
    let busy = std::sync::atomic::AtomicUsize::new(0);
    let failed = || failure.lock().unwrap_or_else(|e| e.into_inner()).is_some();
    let fail = |error: String| *failure.lock().unwrap_or_else(|e| e.into_inner()) = Some(error);
    std::thread::scope(|scope| {
        for worker in 0..most_workers() {
            let (remaining, kept, busy, strips, content, shots_db, lease, failed, fail) = (&remaining, &kept, &busy, &strips, &content, &shots_db, &lease, &failed, &fail);
            scope.spawn(move || {
                let mut client = match content.open(Access::ReadOnly) {
                    Ok(client) => client,
                    Err(error) => return fail(error.to_string()),
                };
                loop {
                    if failed() || !wait_for_player(pace) {
                        return;
                    }
                    // A worker past the number allowed now waits; the clips the others have go on.
                    if worker >= (pace.workers)() {
                        if remaining.lock().unwrap_or_else(|e| e.into_inner()).is_empty() {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    let clip_id = {
                        let mut remaining = remaining.lock().unwrap_or_else(|e| e.into_inner());
                        if remaining.is_empty() {
                            return;
                        }
                        let at = (pace.wanted)().and_then(|clip| remaining.iter().position(|id| *id == clip)).unwrap_or(0);
                        remaining.remove(at)
                    };
                    busy.fetch_add(1, Ordering::Relaxed);
                    let done = analyse_one(&mut client, &clip_id, local_media, strips, shots_db, lease, pace, busy);
                    busy.fetch_sub(1, Ordering::Relaxed);
                    match done {
                        Ok(true) => {
                            kept.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(false) => {}
                        Err(error) => fail(error),
                    }
                }
            });
        }
    });
    match failure.into_inner().unwrap_or_else(|e| e.into_inner()) {
        Some(error) => Err(error),
        None => Ok(kept.into_inner()),
    }
}

/// Analyses one clip and keeps what it found; `false` when it was skipped or the run stopped.
#[allow(clippy::too_many_arguments)]
fn analyse_one(
    client: &mut qnc_content_store::ContentClient,
    clip_id: &str,
    local_media: &LocalMedia<'_>,
    strips: &Option<(PathBuf, String)>,
    shots_db: &ProjectDbTarget,
    lease: &Option<WriteLease>,
    pace: &Pace<'_>,
    busy: &std::sync::atomic::AtomicUsize,
) -> Result<bool, String> {
    let Some(clip) = client.read(clip_id)? else { return Ok(false) };
    let started = Instant::now();
    let original = clip.clip.snapshot.metadata.original.media_uri.clone();
    let Some(path) = local_media(&original) else {
        log(format!("shot-analysis clip={clip_id} skipped: the original is not on this computer"));
        return Ok(false);
    };
    let found = analyse(clip_id, &clip.clip.snapshot, &path, pace);
    if pace.cancel.load(Ordering::Relaxed) {
        return Ok(false);
    }
    let record = match found {
        Ok(found) => {
            let strip = strips.as_ref().and_then(|(folder, base)| match write_strip(folder, base, clip_id, &found.decoded) {
                Ok(strip) => Some(strip),
                Err(error) => {
                    log(format!("shot-analysis clip={clip_id} strip not kept: {error}"));
                    None
                }
            });
            ClipShots { clip_id: clip_id.to_string(), media_uri: original, pictures: found.pictures, shots: found.shots, parts: found.parts, error: String::new(), took_ms: 0, strip }
        }
        Err(error) => ClipShots { clip_id: clip_id.to_string(), media_uri: original, pictures: 0, shots: Vec::new(), parts: Vec::new(), error, took_ms: 0, strip: None },
    };
    let record = ClipShots { took_ms: started.elapsed().as_millis() as u64, ..record };
    log(format!(
        "shot-analysis clip={clip_id} pictures={} shots={} parts={} ms={} workers={} error={}",
        record.pictures,
        record.shots.len(),
        record.parts.len(),
        record.took_ms,
        busy.load(Ordering::Relaxed),
        record.error
    ));
    qnc_shot_db::save(shots_db, record, lease.clone())?;
    Ok(true)
}

/// Waits while the player plays; false when the run stops.
fn wait_for_player(pace: &Pace<'_>) -> bool {
    while (pace.player_plays)() {
        if pace.cancel.load(Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    !pace.cancel.load(Ordering::Relaxed)
}

/// The shots of one clip and how many pictures they came from; an error when its original
/// cannot be read this way.
fn analyse(clip_id: &str, snapshot: &qnc_media_records::Snapshot, path: &Path, pace: &Pace<'_>) -> Result<Found, String> {
    let codec = qnc_filmstrip::original_picture_codec(snapshot).ok_or_else(|| qnc_ui_text::text("shots.codec_not_read").to_string())?;
    let targets = qnc_filmstrip::original_picture_frames(snapshot).unwrap_or_default();
    let found = analyse_file_with(clip_id, codec, path, pace, &targets)?;
    // The filmstrip's thumbnails from the pictures decoded here, for the filmstrip worker of this
    // process (user 2026-11-18: AI shots and the filmstrip in one pass).
    if let Some(jpegs) = found.filmstrip.clone() {
        qnc_filmstrip::handed::hand(clip_id, jpegs);
    }
    Ok(found)
}

/// What the analysis of one clip found.
#[derive(Debug, Clone, PartialEq)]
pub struct Found {
    /// How many pictures it looked at.
    pub pictures: u32,
    pub shots: Vec<ShotRecord>,
    pub parts: Vec<PartRecord>,
    /// The filmstrip's thumbnails (JPEG, `qnc_filmstrip::handed::THUMB_SIZE`), one for each place
    /// asked, from the decoded picture nearest it; `None` when none was asked or one is missing.
    pub filmstrip: Option<Vec<Vec<u8>>>,
    /// Every picture decoded, in the order of the clip: its frame and its JPEG (`PICTURE` size).
    pub decoded: Vec<(u64, Vec<u8>)>,
}

/// The shots and usable parts of one MXF or MP4 original at `path` whose pictures are `codec`
/// (`h264`, `mpeg2video`; an MP4's own index names its codec); the file is only read.
pub fn analyse_file(clip_id: &str, codec: &str, path: &Path, pace: &Pace<'_>) -> Result<Found, String> {
    analyse_file_with(clip_id, codec, path, pace, &[])
}

/// As `analyse_file`, also making the filmstrip's thumbnails for the frames `filmstrip` (its places)
/// from the pictures it decodes: no picture is decoded twice.
pub fn analyse_file_with(clip_id: &str, codec: &str, path: &Path, pace: &Pace<'_>, filmstrip: &[u64]) -> Result<Found, String> {
    let mut keys = KeyPictures::open(path)?.ok_or_else(|| qnc_ui_text::text("shots.no_index").to_string())?;
    let codec = keys.codec().unwrap_or(codec);
    let (frames, rate) = (keys.frames, keys.rate);
    // Every intra picture (about two a second for a camera's long GOP), at most
    // `MOST_A_SECOND` a second (an all-intra original); its frame is where it is shown.
    let least_apart = (rate / MOST_A_SECOND).round().max(1.0) as u64;
    let mut entries: Vec<(usize, u64)> = Vec::new();
    for &(id, shown) in &keys.keys {
        if entries.last().is_none_or(|&(_, last)| shown >= last + least_apart) {
            entries.push((id, shown));
        }
    }
    let extractor = qnc_decoder_catalog::installed_filmstrip_extractor().map_err(|e| e.to_string())?.ok_or_else(|| qnc_ui_text::text("shots.no_decoder").to_string())?;
    // The picture each filmstrip place takes: the one looked at nearest it (an intra picture, as
    // the filmstrip itself takes); its decoded JPEG is kept as it is, the filmstrip's size.
    let nearest: Vec<usize> = if entries.is_empty() { Vec::new() } else { filmstrip.iter().map(|&target| nearest_entry(&entries, target)).collect() };
    let mut kept: std::collections::HashMap<usize, Vec<u8>> = Default::default();
    let mut every_jpeg: Vec<(u64, Vec<u8>)> = Vec::with_capacity(entries.len());
    let temp = scratch_dir(clip_id);
    let result: Result<Vec<qnc_shot_detect::Picture>, String> = (|| {
        let mut pictures = Vec::with_capacity(entries.len());
        let batches: Vec<&[(usize, u64)]> = entries.chunks(BATCH).collect();
        // Several batches at once, each decoded by its own decoder call in a folder of its own;
        // the pictures are read one after the other (the file, one reader), the decoding is what
        // takes the time.
        for (g, group) in batches.chunks(decoders()).enumerate() {
            if !wait_for_player(pace) {
                return Err(qnc_ui_text::text("shots.stopped").into());
            }
            let _ = fs::remove_dir_all(&temp);
            let mut work = Vec::with_capacity(group.len());
            for (b, batch) in group.iter().enumerate() {
                let folder = temp.join(format!("b{b}"));
                fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                let mut inputs = Vec::with_capacity(batch.len());
                for (n, &(id, _)) in batch.iter().enumerate() {
                    let picture = keys.picture(id)?;
                    let input = folder.join(format!("{n:03}.{codec}"));
                    fs::write(&input, picture).map_err(|e| e.to_string())?;
                    inputs.push(input);
                }
                work.push((folder, inputs));
            }
            let decoded: Vec<Result<(), String>> = std::thread::scope(|scope| {
                let running: Vec<_> = work
                    .iter()
                    .map(|(folder, inputs)| scope.spawn(|| extractor.decode_pictures_with_cancel(codec, inputs, PICTURE, folder, pace.cancel).map_err(|e| e.to_string())))
                    .collect();
                running.into_iter().map(|thread| thread.join().unwrap_or_else(|_| Err("a decoder thread failed".into()))).collect()
            });
            for (b, (((folder, _), batch), decoded)) in work.iter().zip(group).zip(decoded).enumerate() {
                decoded?;
                for (n, &(_, shown)) in batch.iter().enumerate() {
                    let file = folder.join(format!("{n:03}.jpg"));
                    let Ok(image) = image::open(&file) else { continue };
                    let entry = (g * decoders() + b) * BATCH + n;
                    // Every picture's JPEG is kept (the clip's strip); the filmstrip's places take theirs.
                    if let Ok(jpeg) = fs::read(&file) {
                        if nearest.contains(&entry) {
                            kept.insert(entry, jpeg.clone());
                        }
                        every_jpeg.push((shown, jpeg));
                    }
                    let grey = image.into_luma8();
                    pictures.push(qnc_shot_detect::Picture { frame: shown, width: grey.width() as usize, height: grey.height() as usize, luma: grey.into_raw() });
                }
            }
        }
        Ok(pictures)
    })();
    let _ = fs::remove_dir_all(&temp);
    let pictures = result?;
    if pictures.is_empty() {
        return Err(qnc_ui_text::text("shots.no_pictures").into());
    }
    let count = pictures.len() as u32;
    let found = qnc_shot_detect::shots(pictures.clone(), frames);
    let parts = qnc_shot_detect::parts(pictures, &found, rate)
        .into_iter()
        .map(|part| PartRecord { from: part.from, to: part.to, camera: part.camera.name().into(), key_frame: part.key_frame })
        .collect();
    let shots = found
        .into_iter()
        .map(|shot| ShotRecord {
            from: shot.from,
            to: shot.to,
            looks: shot.looks.iter().map(|look| look.name().to_string()).collect(),
            key_frame: shot.key_frame,
            brightness: shot.brightness,
            sharpness: shot.sharpness,
            motion: shot.motion,
        })
        .collect();
    let filmstrip = (!nearest.is_empty()).then(|| nearest.iter().map(|entry| kept.get(entry).cloned()).collect::<Option<Vec<_>>>()).flatten();
    Ok(Found { pictures: count, shots, parts, filmstrip, decoded: every_jpeg })
}

/// The entry whose picture is shown nearest `target`.
fn nearest_entry(entries: &[(usize, u64)], target: u64) -> usize {
    entries.iter().enumerate().min_by_key(|(_, &(_, shown))| shown.abs_diff(target)).map_or(0, |(at, _)| at)
}

fn scratch_dir(clip_id: &str) -> PathBuf {
    let name: String = clip_id.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect();
    std::env::temp_dir().join("qnc-shot-analysis").join(format!("{name}-{}", std::process::id()))
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

/// The folder of the project's AI shots on this computer and its URI; `None` (logged) when the
/// project has none or is not on this computer.
fn strip_folder(reader: &SettingsReader, settings: &WorkSettings) -> Option<(PathBuf, String)> {
    let found = reader
        .local_workspace_dir(settings)
        .map_err(|e| e.to_string())
        .and_then(|dir| dir.ok_or_else(|| qnc_ui_text::text("import.import_needs_local_access_project_folder").to_string()))
        .and_then(|dir| settings.ai_shots_location(&dir).map_err(|e| e.message));
    found.map_err(|error| log(format!("shot-analysis strips not kept: {error}"))).ok()
}

/// The file of the clip's strip in the AI shots folder (its own folder, beside the shots' posters).
pub const STRIP_FILE: &str = "pictures.jpgs";

/// Writes the JPEGs of one clip one after the other into `<folder>/<clip>/pictures.jpgs` and
/// says where each one is; a clip analysed again writes its file anew.
pub fn write_strip(folder: &Path, base_uri: &str, clip_id: &str, jpegs: &[(u64, Vec<u8>)]) -> Result<PictureStrip, String> {
    let dir = folder.join(clip_id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut bytes = Vec::with_capacity(jpegs.iter().map(|(_, jpeg)| jpeg.len()).sum());
    let mut pictures = Vec::with_capacity(jpegs.len());
    for (frame, jpeg) in jpegs {
        pictures.push(StripPicture { frame: *frame, offset: bytes.len() as u64, len: u32::try_from(jpeg.len()).map_err(|e| e.to_string())? });
        bytes.extend_from_slice(jpeg);
    }
    fs::write(dir.join(STRIP_FILE), &bytes).map_err(|e| e.to_string())?;
    Ok(PictureStrip { uri: format!("{}/{clip_id}/{STRIP_FILE}", base_uri.trim_end_matches('/')), pictures })
}

#[cfg(test)]
mod strip_tests {
    use super::*;

    #[test]
    fn every_picture_is_found_again_by_its_place_in_the_strip() {
        let dir = std::env::temp_dir().join(format!("qnc-strip-test-{}", std::process::id()));
        let jpegs = vec![(0, vec![1u8, 2, 3]), (25, vec![4, 5]), (50, vec![6, 7, 8, 9])];
        let strip = write_strip(&dir, "qnc://local/source/project-p/file/products/ai_shots/", "c1", &jpegs).unwrap();
        assert_eq!(strip.uri, "qnc://local/source/project-p/file/products/ai_shots/c1/pictures.jpgs");
        let bytes = fs::read(dir.join("c1").join(STRIP_FILE)).unwrap();
        for (picture, (frame, jpeg)) in strip.pictures.iter().zip(&jpegs) {
            assert_eq!(picture.frame, *frame);
            assert_eq!(&bytes[picture.offset as usize..][..picture.len as usize], jpeg.as_slice());
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
