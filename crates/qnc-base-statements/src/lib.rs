//! The statements of an Ingest base in the project that took it (user 2026-11-24: "izjave mogu
//! ostati u bazi... kad se kreira projekt, to se kopira i u projekt"). A statement is light: its
//! clip, IN/OUT, parts, caption and the stills of its IN and OUT. A project made from a base (Script,
//! breaking news) Selects the base as a card, so its clips have ids of their own: a statement is
//! copied once its clip (found by name) is imported in the project, with the base's stills (the
//! base keeps them). A statement already in the project (same clip, IN and OUT) is not copied
//! again, so this may run after every import. The base is only read.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use qnc_db_broker::ProjectDbTarget;

pub const MODULE_ID: &str = "qnc.module.base-statements";

/// What a copy did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Copied {
    /// Statements written into the project now.
    pub copied: usize,
    /// Statements whose clip is not imported in the project yet (a later run copies them).
    pub waiting: usize,
}

/// The database of the project (or base) `project_id`.
fn target_of(root: &Path, project_id: &str) -> Result<(ProjectDbTarget, qnc_content_read::ContentReader), String> {
    let reader = qnc_work_settings::SettingsReader::from_root(root).map_err(|error| error.to_string())?.with_project(project_id);
    let settings = qnc_active_project_read::ActiveProjectReader::from_settings_reader(reader.clone()).read().map_err(|error| error.to_string())?.settings;
    let target = ProjectDbTarget::for_project(&reader, &settings)?;
    let content = qnc_content_read::ContentReader::for_project(&reader, &settings)?;
    Ok((target, content))
}

/// Copies into the project `story_id` the statements of the `bases` it took (their ids).
pub fn copy(root: &Path, story_id: &str, bases: &[String]) -> Result<Copied, String> {
    let bases = bases.to_vec();
    if bases.is_empty() {
        return Ok(Copied::default());
    }
    let (story, content) = target_of(root, story_id)?;
    let imported: HashMap<String, String> = content.summaries()?.into_iter().filter(|clip| clip.imported).map(|clip| (clip.name, clip.clip_id)).collect();
    let mut there: HashSet<(String, u64, u64)> = qnc_virtual_shots::list_shorts(&story)?.into_iter().map(|shot| (shot.clip_id, shot.in_frame, shot.out_frame)).collect();
    let mut done = Copied::default();
    for base in bases {
        let (base_target, base_content) = target_of(root, &base)?;
        let base_names: HashMap<String, String> = base_content.summaries()?.into_iter().map(|clip| (clip.clip_id, clip.name)).collect();
        for shot in qnc_virtual_shots::list_shorts(&base_target)?.into_iter().filter(|shot| !shot.b_roll) {
            let in_story = |clip_id: &str| base_names.get(clip_id).and_then(|name| imported.get(name)).cloned();
            let Some(clip) = in_story(&shot.clip_id) else {
                done.waiting += 1;
                continue;
            };
            let parts: Option<Vec<(String, u64, u64, qnc_virtual_shots::JoinKind)>> = shot.parts.iter().map(|part| in_story(&part.clip_id).map(|clip| (clip, part.in_frame, part.out_frame, part.join))).collect();
            let Some(parts) = parts else {
                done.waiting += 1;
                continue;
            };
            if !there.insert((clip.clone(), shot.in_frame, shot.out_frame)) {
                continue;
            }
            let saved = qnc_virtual_shots::save_short_now(&story, story_id, &clip, &shot.name, shot.in_frame, shot.out_frame, ("", ""))?;
            if let (Some(in_uri), Some(out_uri)) = (shot.in_still_uri.clone(), shot.out_still_uri.clone()) {
                qnc_virtual_shots::publish_stills_now(&story, &saved.shot_id, Ok((in_uri, out_uri)))?;
            }
            if !shot.caption_name.is_empty() || !shot.caption_role.is_empty() {
                qnc_virtual_shots::set_caption_now(&story, &saved.shot_id, &shot.caption_name, &shot.caption_role)?;
            }
            for (clip, in_frame, out_frame, join) in parts {
                qnc_virtual_shots::add_part_now(&story, &saved.shot_id, &clip, in_frame, out_frame, join)?;
            }
            done.copied += 1;
        }
    }
    Ok(done)
}
