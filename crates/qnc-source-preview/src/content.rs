//! Read-only adapters that feed the player and the timeline from public
//! contracts of the active project only: the saved clip with its media record
//! (`qnc-content-store`, read-only) and the content views (`qnc-content-read`).
//! Nothing is read from a host database, so a copied project plays anywhere.

use qnc_content_read::ContentReader;
use qnc_timeline_assets::TimelineArtifactRead;

/// The clip records of the active project (block `qnc-player-clip-content`).
pub(crate) use qnc_player_clip_content::PlayerClipContent as PlayerContent;

#[derive(Clone)]
pub(crate) struct ArtifactReader {
    pub content: ContentReader,
    pub filmstrip_root_uri: String,
    pub filmstrip_dir: std::path::PathBuf,
    /// The AI shots of the project (ai.db) and their folder: the pictures they kept.
    pub shots: Option<qnc_db_broker::ProjectDbTarget>,
    pub ai_shots: Option<(std::path::PathBuf, String)>,
}

impl TimelineArtifactRead for ArtifactReader {
    fn read_filmstrip(
        &self,
        clip_id: &str,
    ) -> Result<Option<qnc_filmstrip::FilmstripArtifactRecord>, String> {
        self.content.filmstrip(clip_id)
    }

    fn read_wave(&self, clip_id: &str) -> Result<Option<qnc_wave::WaveArtifactRecord>, String> {
        self.content.wave(clip_id)
    }

    fn read_image_bytes(&self, artifact_uri: &str) -> Result<Vec<u8>, String> {
        let artifacts = qnc_filmstrip::LocalFilmstripArtifacts::new(
            &self.filmstrip_root_uri,
            &self.filmstrip_dir,
        )?;
        let path = artifacts.frame_path(artifact_uri)?;
        std::fs::read(&path).map_err(|error| format!("filmstrip frame read failed: {error}"))
    }

    fn read_placed(&self, clip_id: &str) -> Result<Option<(u64, Vec<(u64, Vec<u8>)>)>, String> {
        let (Some(target), Some((folder, base))) = (&self.shots, &self.ai_shots) else { return Ok(None) };
        let Some(shots) = qnc_shot_db::read_clip(target, clip_id)? else { return Ok(None) };
        let (Some(strip), Some(total)) = (shots.strip, shots.shots.last().map(|shot| shot.to)) else { return Ok(None) };
        // The strip is a file of the project's AI shots folder, found by its URI under that folder.
        let relative = strip.uri.strip_prefix(base.trim_end_matches('/')).map(|rest| rest.trim_start_matches('/')).ok_or("the strip is not in the AI shots folder")?;
        if relative.split('/').any(|part| part.is_empty() || part == "..") {
            return Err("the strip is not in the AI shots folder".into());
        }
        let bytes = std::fs::read(relative.split('/').fold(folder.clone(), |path, part| path.join(part))).map_err(|error| error.to_string())?;
        let pictures = strip
            .pictures
            .iter()
            .filter_map(|picture| Some((picture.frame, bytes.get(picture.offset as usize..)?.get(..picture.len as usize)?.to_vec())))
            .collect();
        Ok(Some((total, pictures)))
    }
}
