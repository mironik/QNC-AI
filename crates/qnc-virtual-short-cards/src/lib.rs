//! Public view model for virtual short cards.
//!
//! The rule is owned here: the IN still of a saved virtual short is the poster
//! shown by any form that uses virtual short cards. Forms do not derive that
//! rule, and application roots only pass DB rows and loaded poster rasters.

use std::sync::Arc;

pub const MODULE_ID: &str = "qnc.module.virtual-short-cards";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq)]
pub struct ParentClip {
    pub duration_seconds: f64,
    pub duration_frames: u64,
    pub import_status: String,
    pub imported_media_uri: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VirtualShortCard {
    pub shot_id: String,
    pub clip_id: String,
    pub name: String,
    pub in_frame: u64,
    pub out_frame: u64,
    /// v5 card duration: whole seconds and leftover frames, `seconds:ff`.
    pub duration_label: String,
    pub import_status: String,
    pub imported_media_uri: String,
    pub poster_uri: Option<String>,
    pub poster_image: Option<Arc<qnc_image_assets::RgbaImage>>,
    /// The shot of a cover: the B-roll tab shows it, the Virtual tab does not.
    pub b_roll: bool,
    /// Who speaks and their role (the caption the director puts in, user 2026-11-19).
    pub caption_name: String,
    pub caption_role: String,
    /// The parts after its own IN/OUT (a statement of several parts, user 2026-11-19).
    pub parts: Vec<qnc_virtual_shots::ShotPart>,
}

/// The chosen SubClip (not a B-roll shot) as `(shot, clip, (in, out))`, for its fine tuning.
pub fn chosen<'a>(cards: &'a [VirtualShortCard], shot_id: Option<&str>) -> Option<(&'a str, &'a str, (u64, u64))> {
    let card = cards.iter().find(|card| Some(card.shot_id.as_str()) == shot_id && !card.b_roll)?;
    Some((card.shot_id.as_str(), card.clip_id.as_str(), (card.in_frame, card.out_frame)))
}

/// The chosen statement of one range (`chosen`), for Update and its fine tuning on the source
/// timeline (user 2026-11-24: Update with a composite statement open wrote the shown part's IN/OUT
/// over the whole statement; a composite one is edited on its own timeline).
pub fn chosen_single<'a>(cards: &'a [VirtualShortCard], shot_id: Option<&str>) -> Option<(&'a str, &'a str, (u64, u64))> {
    cards.iter().any(|card| Some(card.shot_id.as_str()) == shot_id && card.parts.is_empty()).then(|| chosen(cards, shot_id)).flatten()
}

/// The parts of the chosen statement in the clip shown, its own IN/OUT first, while it has parts
/// (user 2026-11-19: a compound statement lights all its parts in the transcript; from the
/// statement's record, the text is the transcript's).
pub fn chosen_parts(cards: &[VirtualShortCard], shot_id: Option<&str>, shown_clip: Option<&str>) -> Option<(String, Vec<(u64, u64)>)> {
    let card = cards.iter().find(|card| Some(card.shot_id.as_str()) == shot_id && !card.b_roll && !card.parts.is_empty())?;
    let shown = shown_clip?;
    let own = (card.clip_id == shown).then_some((card.in_frame, card.out_frame));
    let parts = card.parts.iter().filter(|part| part.clip_id == shown).map(|part| (part.in_frame, part.out_frame));
    Some((shown.to_string(), own.into_iter().chain(parts).collect()))
}

/// The IN/OUT of the chosen statement while its clip is the one shown: the source timeline
/// enlarges them (user 2026-11-09).
pub fn shown_marks(cards: &[VirtualShortCard], shot_id: Option<&str>, shown_clip: Option<&str>) -> Option<(u64, u64)> {
    chosen(cards, shot_id).filter(|(_, clip, _)| Some(*clip) == shown_clip).map(|(_, _, marks)| marks)
}

/// The card of `shot_id` to open, or the text code of why it cannot be: it is gone or its clip
/// is not in the pool.
pub fn to_open<'a>(cards: &'a [VirtualShortCard], clips: &[qnc_clip_row::ClipRow], shot_id: &str) -> Result<&'a VirtualShortCard, &'static str> {
    let card = cards.iter().find(|card| card.shot_id == shot_id).ok_or("story.msg.subclip_not_found")?;
    clips.iter().any(|clip| clip.clip_id == card.clip_id).then_some(card).ok_or("ingest.msg.clip_not_found")
}

/// The short's card in the media pool (piece `qnc-media-card`).
impl qnc_clip_row::PoolItem for VirtualShortCard {
    fn card_id(&self) -> &str { &self.shot_id }
    fn card_title(&self) -> &str { &self.name }
    fn card_poster(&self) -> (Option<&str>, Option<&qnc_image_assets::RgbaImage>) { (self.poster_uri.as_deref(), self.poster_image.as_deref()) }
    fn card_duration_label(&self) -> &str { &self.duration_label }
    fn card_import(&self) -> (&str, &str) { (&self.import_status, &self.imported_media_uri) }
}

pub fn build_card(
    row: qnc_virtual_shots::ShortClip,
    parent: Option<ParentClip>,
) -> VirtualShortCard {
    VirtualShortCard {
        shot_id: row.shot_id,
        clip_id: row.clip_id,
        name: row.name,
        in_frame: row.in_frame,
        out_frame: row.out_frame,
        duration_label: duration_label(row.out_frame.saturating_sub(row.in_frame), parent.as_ref()),
        import_status: parent
            .as_ref()
            .map(|clip| clip.import_status.clone())
            .unwrap_or_default(),
        imported_media_uri: parent
            .map(|clip| clip.imported_media_uri)
            .unwrap_or_default(),
        poster_uri: row.in_still_uri,
        poster_image: None,
        b_roll: row.b_roll,
        caption_name: row.caption_name,
        caption_role: row.caption_role,
        parts: row.parts,
    }
}

/// The cards of the rows read from the database, each with its parent clip of the pool, the
/// posters already loaded kept (audit 2026-10-25: this was in the Story application).
pub fn cards(rows: Vec<qnc_virtual_shots::ShortClip>, clips: &[qnc_clip_row::ClipRow], previous: &[VirtualShortCard]) -> Vec<VirtualShortCard> {
    let mut cards: Vec<VirtualShortCard> = rows
        .into_iter()
        .map(|row| {
            let parent = clips.iter().find(|clip| clip.clip_id == row.clip_id).map(|clip| ParentClip {
                duration_seconds: clip.duration_seconds,
                duration_frames: clip.duration_frames,
                import_status: clip.import_status.clone(),
                imported_media_uri: clip.imported_media_uri.clone(),
            });
            build_card(row, parent)
        })
        .collect();
    preserve_loaded_posters(&mut cards, previous);
    // A statement still being written stays until its row is read (user 2026-11-15).
    let saving = previous.iter().filter(|old| is_saving(old) && !cards.iter().any(|card| same_cut(card, old)));
    cards.extend(saving.cloned().collect::<Vec<_>>());
    cards
}

/// The id of a card shown while its statement is written, before the database has its row; with
/// the monitor's IN picture as poster (that picture is the statement's own poster too).
const SAVING: &str = "saving:";
const SAVING_IN: &str = "saving-in:";

fn is_saving(card: &VirtualShortCard) -> bool {
    card.shot_id.starts_with(SAVING) || card.shot_id.starts_with(SAVING_IN)
}

fn same_cut(card: &VirtualShortCard, other: &VirtualShortCard) -> bool {
    card.clip_id == other.clip_id && card.in_frame == other.in_frame && card.out_frame == other.out_frame && !card.b_roll && !other.b_roll
}

/// A statement asked to be saved shows at once (user 2026-11-15: "dugo se čeka poster kreirane
/// izjave, ne čekaj zapis u bazu"): a card of the clip and its IN/OUT with the picture the monitor
/// showed at the IN (`in_picture`), else the clip's poster; its row replaces it once it is read.
/// Nothing is written here. A card of the same cut already there is left alone.
pub fn saving(cards: &mut Vec<VirtualShortCard>, clips: &[qnc_clip_row::ClipRow], clip_id: Option<&str>, marks: Option<(u64, u64)>, in_picture: Option<Arc<qnc_image_assets::RgbaImage>>) {
    let (Some(clip), Some((in_frame, out_frame))) = (clip_id.and_then(|id| clips.iter().find(|clip| clip.clip_id == id)), marks) else {
        return;
    };
    if cards.iter().any(|card| !card.b_roll && card.clip_id == clip.clip_id && (card.in_frame, card.out_frame) == (in_frame, out_frame)) {
        return;
    }
    let parent = ParentClip { duration_seconds: clip.duration_seconds, duration_frames: clip.duration_frames, import_status: clip.import_status.clone(), imported_media_uri: clip.imported_media_uri.clone() };
    let kind = if in_picture.is_some() { SAVING_IN } else { SAVING };
    cards.push(VirtualShortCard {
        shot_id: format!("{kind}{}:{in_frame}-{out_frame}", clip.clip_id),
        clip_id: clip.clip_id.clone(),
        name: clip.name.clone(),
        in_frame,
        out_frame,
        duration_label: duration_label(out_frame.saturating_sub(in_frame), Some(&parent)),
        import_status: parent.import_status,
        imported_media_uri: parent.imported_media_uri,
        poster_uri: None,
        poster_image: in_picture.or_else(|| clip.thumb_image.clone()),
        b_roll: false,
        caption_name: String::new(),
        caption_role: String::new(),
        parts: Vec::new(),
    });
}

/// The cards shown while their statement was written, when it was not (its error is said).
pub fn drop_saving(cards: &mut Vec<VirtualShortCard>) {
    cards.retain(|card| !is_saving(card));
}

pub fn preserve_loaded_posters(cards: &mut [VirtualShortCard], previous: &[VirtualShortCard]) {
    for card in cards {
        card.poster_image = previous
            .iter()
            .find(|old| old.shot_id == card.shot_id && old.poster_uri == card.poster_uri)
            // A statement just written keeps the IN picture shown while it was written until its
            // own still is read (the same picture).
            .or_else(|| previous.iter().find(|old| old.shot_id.starts_with(SAVING_IN) && same_cut(card, old)))
            .or_else(|| previous.iter().find(|old| old.shot_id == card.shot_id && old.in_frame == card.in_frame && old.poster_uri.is_none() && old.poster_image.is_some()))
            .and_then(|old| old.poster_image.clone());
    }
}

pub fn poster_requests(cards: &[VirtualShortCard]) -> Vec<(String, String)> {
    cards
        .iter()
        .filter(|card| card.poster_image.is_none())
        .filter_map(|card| Some((card.shot_id.clone(), card.poster_uri.clone()?)))
        .collect()
}

pub fn apply_poster(
    cards: &mut [VirtualShortCard],
    shot_id: &str,
    image: Arc<qnc_image_assets::RgbaImage>,
) -> bool {
    let Some(card) = cards.iter_mut().find(|card| card.shot_id == shot_id) else {
        return false;
    };
    card.poster_image = Some(image);
    true
}

fn duration_label(frames: u64, clip: Option<&ParentClip>) -> String {
    let Some(clip) = clip else {
        return "0:00".into();
    };
    if clip.duration_seconds <= 0.0 || clip.duration_frames == 0 {
        return "0:00".into();
    }
    let fps = (clip.duration_frames as f64 / clip.duration_seconds)
        .round()
        .max(1.0) as u64;
    let seconds = frames / fps;
    let rem = frames % fps;
    format!("{seconds}:{rem:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> qnc_virtual_shots::ShortClip {
        qnc_virtual_shots::ShortClip {
            shot_id: "shot-1".into(),
            clip_id: "clip-a".into(),
            in_frame: 25,
            out_frame: 75,
            name: "Mironik 001".into(),
            in_still_uri: Some("qnc://local/project/p1/virtual_shorts/shot-1/in.jpg".into()),
            out_still_uri: Some("qnc://local/project/p1/virtual_shorts/shot-1/out.jpg".into()),
            still_status: "ready".into(),
            b_roll: false,
            caption_name: String::new(),
            caption_role: String::new(),
            parts: Vec::new(),
        }
    }

    #[test]
    fn a_statement_shows_at_once_with_the_in_picture_and_its_row_keeps_it() {
        let clip = qnc_clip_row::ClipRow { clip_id: "clip-a".into(), name: "Mironik 001".into(), duration_seconds: 10.0, duration_frames: 250, ..Default::default() };
        let picture = Arc::new(qnc_image_assets::RgbaImage { size: [1, 1], pixels: vec![1, 2, 3, 255], content_key: 7 });
        let mut shown = Vec::new();
        saving(&mut shown, std::slice::from_ref(&clip), Some("clip-a"), Some((25, 75)), Some(picture.clone()));
        assert_eq!(shown.len(), 1, "the card is there before the database has its row");
        assert_eq!(shown[0].poster_image, Some(picture.clone()));
        assert_eq!(cards(Vec::new(), std::slice::from_ref(&clip), &shown).len(), 1, "a read before the row keeps it");
        let written = qnc_virtual_shots::ShortClip { in_still_uri: None, out_still_uri: None, still_status: "pending".into(), ..row() };
        let read = cards(vec![written], std::slice::from_ref(&clip), &shown);
        assert_eq!(read.len(), 1, "its row takes its place");
        assert_eq!(read[0].shot_id, "shot-1");
        assert_eq!(read[0].poster_image, Some(picture), "with the IN picture until its own still is read");
        drop_saving(&mut shown);
        assert!(shown.is_empty());
    }

    #[test]
    fn update_and_fine_tuning_take_only_a_statement_of_one_range() {
        let clip = qnc_clip_row::ClipRow { clip_id: "clip-a".into(), name: "Mironik 001".into(), duration_seconds: 10.0, duration_frames: 250, ..Default::default() };
        let part = qnc_virtual_shots::ShotPart { clip_id: "clip-a".into(), in_frame: 100, out_frame: 150, join: qnc_virtual_shots::JoinKind::Smooth };
        let shown = cards(vec![row(), qnc_virtual_shots::ShortClip { shot_id: "shot-2".into(), parts: vec![part], ..row() }], std::slice::from_ref(&clip), &[]);
        assert!(chosen_single(&shown, Some("shot-1")).is_some());
        assert!(chosen_single(&shown, Some("shot-2")).is_none(), "a composite statement is edited on its own timeline");
        assert!(chosen(&shown, Some("shot-2")).is_some(), "a part may still be added to it");
    }

    #[test]
    fn in_still_is_the_public_card_poster() {
        let card = build_card(
            row(),
            Some(ParentClip {
                duration_seconds: 10.0,
                duration_frames: 250,
                import_status: "imported".into(),
                imported_media_uri: "qnc://media".into(),
            }),
        );
        assert_eq!(
            card.poster_uri.as_deref(),
            Some("qnc://local/project/p1/virtual_shorts/shot-1/in.jpg")
        );
        assert_eq!(card.duration_label, "2:00");
    }

    #[test]
    fn poster_requests_use_shot_identity() {
        let card = build_card(row(), None);
        assert_eq!(
            poster_requests(&[card]),
            vec![(
                "shot-1".to_string(),
                "qnc://local/project/p1/virtual_shorts/shot-1/in.jpg".to_string()
            )]
        );
    }
}
