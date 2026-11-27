//! The pictures on both sides of an edit being moved (user 2026-10-10: a professional
//! trim shows the last picture before and the first after every moved marker). For a
//! slot that is four: the last of the slot before, its own first and last, the first of
//! the slot after. They come from the draft, so they show the program as it will be
//! after Enter: the moved slot's cover goes with it, a neighbouring slot shows its own
//! cover where it still has it, else the picture of the segment, Off is black.
//! Pure: the pictures are named (clip and source frame); someone else decodes them.

use std::sync::Arc;

use crate::{source_at, CoverSpan, SegmentsView, Slot};

/// A picture of the program: a frame of a clip.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StillPicture {
    pub clip_id: String,
    pub source_frame: u64,
}

/// One picture beside a moved marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditStill {
    /// Program frame of the marker in the draft.
    pub marker_frame: u64,
    /// The first picture from the marker; else the last one before it.
    pub after: bool,
    /// `None`: no picture there (an Off segment), drawn black.
    pub picture: Option<StillPicture>,
    /// The decoded picture, once it is there.
    pub image: Option<Arc<qnc_image_assets::RgbaImage>>,
    /// Texture identity of the picture.
    pub id: String,
    /// What the picture is, e.g. "Slot - first" (shown under it).
    pub label: String,
    /// The picture at the cut being moved (user 2026-10-13: marked red in the zoom).
    pub cut: bool,
    /// The picture is the first or the last of its clip: the edit cannot go further
    /// that way (user 2026-10-13: a red line on that side in the zoom).
    pub clip_edge: Option<ClipEdge>,
    /// How many frames the picture's clip has, when known (nothing beyond is decoded).
    pub clip_frames: Option<u64>,
    /// The draft asked a picture past the end of its clip (it is shown at the last one).
    pub past_end: bool,
    /// What a click on the picture does (user 2026-10-29: in a SubSegment's zoom the left
    /// picture takes the OUT before the cut, the right one the IN after it): a catalog action
    /// and its target; `None`: nothing.
    pub click: Option<StillAction>,
    /// What a right click on the picture (outside its strip) does (user 2026-11-05: "a right
    /// click on the filmstrip not covered by the strip is the marker"); `None`: nothing.
    pub right_click: Option<StillAction>,
    /// The length strip of the slot the picture shows, as on the timeline (user 2026-11-05: a
    /// left click on it selects the slot, in the timeline and in the zoom); `None`: no strip.
    pub strip: Option<StillStrip>,
    /// The place shows the player itself, the monitor's picture as it plays (user 2026-11-20:
    /// "in the zoom the player moves into the middle window"), not a decoded still.
    pub live: bool,
    /// The middle place of a zoom (docs/zoom-skeleton.md [B]): the player of the whole timeline, or,
    /// while a side is taken, the picture where the taken edge was before it moved. Never a side of
    /// the cut; its `cut` says it is the active place (a zoom opened with nothing taken, framed).
    pub middle: bool,
    /// The elements of an edge picture in the Story's zoom (AGENTS 3a "Model reza i zoom"): the
    /// strip ripples, the M bar rolls the whole cut, the picture and A1/A2 roll that track.
    /// `None`: a plain picture (the source timeline's).
    pub elements: Option<StillElements>,
}

/// One element of a zoom picture: what a click on it does and whether it is the chosen one (red).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StillElement {
    pub action: StillAction,
    pub chosen: bool,
}

/// The elements of an edge picture (user 2026-11-26): from the top the picture, the slot's strip
/// (`EditStill::strip`), A1 and A2; the M marker is a vertical bar through them on the outer edge
/// (the left edge of IN, the right edge of OUT). One click is one action; only the chosen is red.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StillElements {
    /// The picture is chosen (a red frame); its click is `EditStill::click`.
    pub picture: bool,
    /// The strip is chosen (a red border); its click is the strip's `select`.
    pub strip: bool,
    /// The M marker bar; `None`: no marker there (the locked start or end of the story).
    pub marker: Option<StillElement>,
    /// A1 and A2.
    pub sounds: [Option<StillElement>; 2],
}

/// What is chosen in the zoom of a clip (AGENTS 3a, user 2026-11-26): `right` is the OUT side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipZoomTaken {
    Nothing,
    /// The clip's own IN or OUT: its length alone changes (the neighbours keep theirs).
    Edge { right: bool },
    /// The neighbour's edge at that side (the OUT of the clip before, the IN of the clip after):
    /// the neighbour's length alone changes.
    Neighbour { right: bool },
    /// The M marker on that side, the OUT/IN pair: both lengths change (roll).
    Marker { right: bool },
    /// A1 or A2 of that M marker: the roll of that sound (J/L).
    Sound { right: bool, lane: u8 },
}

/// One side of a clip's zoom: the clip's own edge (its picture, the frame of the axis it stands
/// on, what a click on it takes) and the neighbour there with its edge picture and take (`None`:
/// no neighbour). `marker`: the M marker, the OUT/IN pair (`None`: none to roll, e.g. a pause);
/// `sounds`: that M marker's A1/A2.
#[derive(Debug, Clone)]
pub struct ClipZoomEdge {
    pub picture: Option<StillPicture>,
    pub frame: u64,
    pub take: StillAction,
    pub neighbour: Option<(Option<StillPicture>, StillAction)>,
    pub marker: Option<StillAction>,
    pub sounds: [Option<StillAction>; 2],
}

/// The zoom of a clip (AGENTS 3a "Model reza i zoom", user 2026-11-27: "maximalno 3 slike"): at
/// most three windows, `[IN] [SLOT] [OUT]`. The slot in the middle is the player with the clip's
/// one length strip; with the neighbour's edge taken (the OUT of the clip before, the IN of the
/// clip after) its picture stands in the left or right window in place of this clip's IN or OUT.
/// The M marker (the pair OUT/IN) is a wide bar on the outer edge; the sound lanes under a side
/// are that M marker's sound. The player is in the window of what is chosen; only that is red.
pub fn clip_zoom_stills(in_side: ClipZoomEdge, out_side: ClipZoomEdge, strip: StillStrip, (playhead, at_playhead): (u64, Option<StillPicture>), taken: ClipZoomTaken) -> Vec<EditStill> {
    let chosen = |which: ClipZoomTaken| taken == which;
    let pair_elements = |side: &ClipZoomEdge, right: bool, picture: bool| StillElements {
        picture,
        strip: false,
        marker: side.marker.clone().map(|action| StillElement { action, chosen: chosen(ClipZoomTaken::Marker { right }) }),
        sounds: [0u8, 1].map(|lane| side.sounds[usize::from(lane)].clone().map(|action| StillElement { action, chosen: chosen(ClipZoomTaken::Sound { right, lane }) })),
    };
    let still = |picture: Option<StillPicture>, frame: u64, after: bool, label: &'static str, take: StillAction, elements: StillElements| {
        let mut still = EditStill::new(frame, after, picture).labelled(&qnc_ui_text::text(label));
        still.click = Some(take);
        still.elements = Some(elements);
        still
    };
    // One side's window: the neighbour's edge while it is taken, else this clip's own edge.
    let side = |edge: &ClipZoomEdge, right: bool| {
        let neighbour = edge.neighbour.clone().filter(|_| chosen(ClipZoomTaken::Neighbour { right }));
        let own_label = if right { "story.still.out_last" } else { "story.still.in_first" };
        match neighbour {
            Some((picture, take)) => still(picture, edge.frame, right, if right { "story.still.in_first" } else { "story.still.out_last" }, take, pair_elements(edge, right, true)),
            None => still(edge.picture.clone(), edge.frame, !right, own_label, edge.take.clone(), pair_elements(edge, right, chosen(ClipZoomTaken::Edge { right }))),
        }
    };
    let live_side = |right: bool| matches!(taken, ClipZoomTaken::Edge { right: r } | ClipZoomTaken::Marker { right: r } | ClipZoomTaken::Sound { right: r, .. } | ClipZoomTaken::Neighbour { right: r } if r == right);
    let mut stills = vec![side(&in_side, false)];
    let mut player_at = live_side(false).then_some(0);
    // The slot: the player with the clip's one length strip; a click on it lets go (the player again).
    let mut middle = EditStill::new(playhead, true, at_playhead).labelled(&qnc_ui_text::text("zoom.player"));
    middle.middle = true;
    let player = StillAction::new("activate_focused_item", None, None);
    middle.click = Some(player.clone());
    middle.strip = Some(StillStrip { select: Some(player), ..strip });
    middle.elements = Some(StillElements::default());
    let slot = stills.len();
    stills.push(middle);
    if live_side(true) {
        player_at = Some(stills.len());
    }
    stills.push(side(&out_side, true));
    let player_at = player_at.unwrap_or(slot);
    for (index, still) in stills.iter_mut().enumerate() {
        still.live = index == player_at;
        still.cut = index == player_at;
    }
    stills
}

/// The length strip under a zoom picture: the slot's length in the colour of its length.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StillStrip {
    /// The slot's length as shown (`seconds:frames`) and in frames.
    pub label: String,
    pub frames: u64,
    /// The colour of its length (set by the panel that knows the bands); `None`: the accent.
    pub fill: Option<[u8; 3]>,
    /// The slot is the chosen one.
    pub chosen: bool,
    /// A left click anywhere on the strip chooses this slot and takes nothing (user 2026-11-26:
    /// the rule of ends and middle is gone; IN and OUT are taken by a click on the pictures).
    /// `None`: the zoom's one pick of the side. A right click takes it, as on the timeline.
    pub select: Option<StillAction>,
    pub take: StillAction,
}

/// Which end of its clip a picture reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipEdge {
    Start,
    End,
}

/// Marks the pictures at the first or the last frame of their clip and brings a picture
/// beyond the end of its clip back to its last frame (a cover that runs past its clip
/// plays to its end; asking a decoder for a picture that is not there only waits).
/// `frames` gives a clip's length in its own frames.
pub fn mark_clip_edges(stills: &mut [EditStill], frames: impl Fn(&str) -> Option<u64>) {
    for still in stills {
        let Some(picture) = still.picture.as_mut() else { continue };
        let Some(length) = frames(&picture.clip_id).filter(|length| *length > 0) else { continue };
        still.clip_frames = Some(length);
        if picture.source_frame + 1 >= length {
            if picture.source_frame + 1 > length {
                picture.source_frame = length - 1;
                still.past_end = true;
                still.id = format!("qnc_edit_still:{}:{}", picture.clip_id, picture.source_frame);
            }
            still.clip_edge = Some(ClipEdge::End);
        } else if picture.source_frame == 0 {
            still.clip_edge = Some(ClipEdge::Start);
        }
    }
}

/// The pictures beside markers moved from `old` to `new` (`moves`), with the stored
/// slots (not the drafted ones).
pub fn edit_stills(view: &SegmentsView, slots: &[Slot], moves: &[(u64, u64)]) -> Vec<EditStill> {
    let moved = |frame: u64| moves.iter().find(|(old, _)| *old == frame).map_or(frame, |(_, new)| *new);
    // A slot moves two markers: the slot before, the slot itself, the slot after.
    let labels: &[[&str; 2]] = if moves.len() == 2 {
        &[[qnc_ui_text::text("story.still.previous_slot_last"), qnc_ui_text::text("story.still.slot_first")], [qnc_ui_text::text("story.still.slot_last"), qnc_ui_text::text("story.still.next_slot_first")]]
    } else {
        &[[qnc_ui_text::text("story.still.before_marker_last"), qnc_ui_text::text("story.still.after_marker_first")]]
    };
    let mut stills = Vec::new();
    let slot = moves.len() == 2;
    for (index, (&(_, new), [before, after])) in moves.iter().zip(labels.iter().chain(std::iter::repeat(&labels[0]))).enumerate() {
        stills.extend(
            crate::CutPictures {
                cut: new,
                before: (new > 0).then(|| (side_picture(view, slots, &moved, new, false), *before)),
                after: (new < view.total_frames).then(|| (side_picture(view, slots, &moved, new, true), *after)),
                // The cut: the picture after a marker; of a slot its own first and last.
                moving_after: !(slot && index == 1),
            }
            .stills(),
        );
    }
    stills
}

/// The pictures of a slot trimmed at one edge by `frames` (user 2026-10-13): the slots
/// after it move with its end, so at its start the last picture of the slot before and
/// its new first one (its cover starts `frames` earlier in its source), at its end its
/// new last picture and the first of the slot after it.
pub fn trim_stills(view: &SegmentsView, slots: &[Slot], slot_id: &str, start: bool, frames: i64) -> Vec<EditStill> {
    let Some(slot) = slots.iter().find(|slot| slot.slot_id == slot_id) else {
        return Vec::new();
    };
    let total = view.total_frames;
    let new_end = slot.end_frame.saturating_add_signed(frames);
    // In the S-Story the story ends with its last clip: the end moves with it.
    let moved = |frame: u64| if frame >= slot.end_frame && (frame < total || view.clips_are_slots) { frame.saturating_add_signed(frames) } else { frame };
    if start {
        let clips = clip_covers(view, slots);
        let cover = view.covers.iter().chain(&clips).find(|cover| cover.slot_id == slot.slot_id);
        let first = match cover {
            Some(cover) => {
                let length = cover.end_frame.saturating_sub(cover.start_frame).max(1) as i64;
                let source = cover.source_out_frame.saturating_sub(cover.source_in_frame) as i64;
                let source_in = (cover.source_in_frame as i64 - frames * source / length).max(0);
                Some(StillPicture { clip_id: cover.clip_id.clone(), source_frame: source_in as u64 })
            }
            None => picture_at(view, &[], &|frame| frame, slot.start_frame),
        };
        crate::CutPictures {
            cut: slot.start_frame,
            before: (slot.start_frame > 0).then(|| (side_picture(view, slots, &|frame| frame, slot.start_frame, false), qnc_ui_text::text("story.still.previous_slot_last"))),
            after: Some((first, qnc_ui_text::text("story.still.slot_new_first"))),
            moving_after: true,
        }
        .stills()
    } else {
        crate::CutPictures {
            cut: new_end,
            before: (new_end > 0).then(|| (side_picture(view, slots, &moved, new_end, false), qnc_ui_text::text("story.still.slot_new_last"))),
            after: (new_end < total).then(|| (side_picture(view, slots, &moved, new_end, true), qnc_ui_text::text("story.still.next_slot_first"))),
            moving_after: false,
        }
        .stills()
    }
}

impl EditStill {
    /// A picture beside the edit at `marker_frame`; `None` is drawn black.
    pub fn new(marker_frame: u64, after: bool, picture: Option<StillPicture>) -> Self {
        let id = picture
            .as_ref()
            .map_or_else(String::new, |p| format!("qnc_edit_still:{}:{}", p.clip_id, p.source_frame));
        Self { marker_frame, after, picture, image: None, id, label: String::new(), cut: false, clip_edge: None, clip_frames: None, past_end: false, click: None, right_click: None, strip: None, live: false, middle: false, elements: None }
    }

    /// The same picture with what it is.
    pub fn labelled(mut self, label: &str) -> Self {
        self.label = label.to_string();
        self
    }

    /// The same picture marked as the one at the cut being moved.
    pub fn at_cut(mut self) -> Self {
        self.cut = true;
        self
    }
}

/// The picture the stored program shows at a program frame (a cover, else the segment;
/// `None` on Off or outside the program).
pub fn program_picture(view: &SegmentsView, frame: u64) -> Option<StillPicture> {
    (frame < view.total_frames).then(|| picture_at(view, &view.slots, &|frame| frame, frame)).flatten()
}

/// The picture on one side of the cut at `marker` of the draft.
fn side_picture(view: &SegmentsView, slots: &[Slot], moved: &dyn Fn(u64) -> u64, marker: u64, after: bool) -> Option<StillPicture> {
    picture_at(view, slots, moved, if after { marker } else { marker - 1 })
}

/// The picture at a program frame of the draft: a cover there, else the segment.
fn picture_at(view: &SegmentsView, slots: &[Slot], moved: &dyn Fn(u64) -> u64, frame: u64) -> Option<StillPicture> {
    if let Some(picture) = view.covers.iter().chain(&clip_covers(view, slots)).find_map(|cover| cover_picture(cover, slots, moved, frame)) {
        return Some(picture);
    }
    let (segment, source_frame) = source_at(view, frame)?;
    // In a compound statement the picture is its part's there.
    if !segment.parts.is_empty() {
        let part = segment.parts.iter().find(|part| part.start_frame <= frame && frame < part.end_frame)?;
        return part.kind.has_base_video().then(|| StillPicture { clip_id: part.clip_id.clone(), source_frame: part.source_in_frame + (frame - part.start_frame) * part.source_out_frame.saturating_sub(part.source_in_frame) / part.duration_frames().max(1) });
    }
    segment.kind.has_base_video().then(|| StillPicture { clip_id: segment.clip_id.clone(), source_frame })
}

/// A cover covers its whole slot, from its start marker to its end marker.
fn cover_picture(cover: &CoverSpan, slots: &[Slot], moved: &dyn Fn(u64) -> u64, frame: u64) -> Option<StillPicture> {
    let slot = slots.iter().find(|slot| slot.slot_id == cover.slot_id);
    let (old_start, old_end) = slot.map_or((cover.start_frame, cover.end_frame), |slot| (slot.start_frame, slot.end_frame));
    let (start, slot_end) = (moved(old_start), moved(old_end));
    // A cover fills its whole slot and plays its source from IN for the slot's length
    // (the store binds it so), at the story's ratio to its source (the stored span).
    let length = cover.end_frame.saturating_sub(cover.start_frame).max(1) as i64;
    if !(start..slot_end).contains(&frame) {
        return None;
    }
    // Roll edit (the store's `roll_cover`): only the start marker moved, so the cover
    // starts that much later or earlier in its source; a slot moved whole keeps it.
    let roll = if start != old_start && slot_end == old_end { start as i64 - old_start as i64 } else { 0 };
    let source = cover.source_out_frame.saturating_sub(cover.source_in_frame) as i64;
    // As far as its source allows: a cover never starts before its first picture.
    let source_in = (cover.source_in_frame as i64 + roll * source / length).max(0);
    let source_frame = source_in + (frame - start) as i64 * source / length;
    Some(StillPicture { clip_id: cover.clip_id.clone(), source_frame: u64::try_from(source_frame).unwrap_or(0) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SegmentKind, SegmentRow};

    fn view() -> (SegmentsView, Vec<Slot>) {
        let row = |id: &str, kind, (start, end): (u64, u64), source_in: u64| SegmentRow {
            segment_id: id.into(),
            kind,
            clip_id: format!("clip-{id}"),
            source_in_frame: source_in,
            source_out_frame: source_in + (end - start),
            a1_source_channel: 0,
            a1_level_db: 0,
            a1_off: false,
            start_frame: start,
            end_frame: end,
            duration_label: String::new(),
            duration_color_key: "under_3",
            selected: false,
            lead: false,
            level_keys: Vec::new(),
            parts: Vec::new(),
        };
        let slot = |id: &str, start, end| Slot { slot_id: id.into(), start_frame: start, end_frame: end, has_cover: id == "s2", selected: false };
        let slots = vec![slot("s1", 0, 15), slot("s2", 15, 33), slot("s3", 33, 40)];
        let view = SegmentsView {
            rows: vec![row("a", SegmentKind::Ton, (0, 20), 100), row("b", SegmentKind::Off, (20, 40), 0)],
            covers: vec![CoverSpan {
                cover_id: "c".into(),
                slot_id: "s2".into(),
                start_frame: 15,
                end_frame: 25,
                selected: false,
                clip_id: "clip-cover".into(),
                source_in_frame: 500,
                source_out_frame: 510,
                a2_source_channel: 1,
                a2_level_db: 0,
                a2_off: false,
                level_keys: Vec::new(),
            }],
            total_frames: 40,
            slots: slots.clone(),
            ..Default::default()
        };
        (view, slots)
    }

    fn pictures(stills: &[EditStill]) -> Vec<Option<(String, u64)>> {
        stills.iter().map(|still| still.picture.clone().map(|p| (p.clip_id, p.source_frame))).collect()
    }

    #[test]
    fn a_moved_slot_shows_four_pictures_from_the_draft() {
        let (view, slots) = view();
        // The slot 15..33 moved two frames right: its cover goes with it.
        let stills = edit_stills(&view, &slots, &[(15, 17), (33, 35)]);
        assert_eq!(
            pictures(&stills),
            vec![
                Some(("clip-a".into(), 116)), // the slot before ends on the Ton segment
                Some(("clip-cover".into(), 500)), // the cover's first picture, at 17 now
                Some(("clip-cover".into(), 517)), // the cover fills the slot: its last picture
                None,                             // and so is the first of the slot after
            ]
        );
        assert_eq!(stills.iter().map(|s| (s.marker_frame, s.after)).collect::<Vec<_>>(), vec![(17, false), (17, true), (35, false), (35, true)]);
    }

    #[test]
    fn a_moved_marker_shows_the_picture_before_and_after_it() {
        let (view, slots) = view();
        let stills = edit_stills(&view, &slots, &[(15, 12)]);
        // Roll edit: the covered slot starts 3 earlier, so does its cover in its source.
        assert_eq!(pictures(&stills), vec![Some(("clip-a".into(), 111)), Some(("clip-cover".into(), 497))]);
        let stills = edit_stills(&view, &slots, &[(15, 18)]);
        assert_eq!(pictures(&stills), vec![Some(("clip-a".into(), 117)), Some(("clip-cover".into(), 503))]);
    }
}

#[cfg(test)]
mod clip_edge_tests {
    use super::*;

    #[test]
    fn a_picture_at_or_past_the_end_of_its_clip_is_marked_and_held_on_its_last_frame() {
        let still = |frame| EditStill::new(10, true, Some(StillPicture { clip_id: "c".into(), source_frame: frame }));
        let mut stills = vec![still(0), still(50), still(99), still(120)];
        mark_clip_edges(&mut stills, |_| Some(100));
        let edges: Vec<_> = stills.iter().map(|still| still.clip_edge).collect();
        assert_eq!(edges, vec![Some(ClipEdge::Start), None, Some(ClipEdge::End), Some(ClipEdge::End)]);
        assert_eq!(stills[3].picture.as_ref().unwrap().source_frame, 99, "past the end: the last frame");
    }
}

/// In the S-Story (user 2026-10-31: "off = slot, ton = slot") the picture clip of every slot,
/// as the cover of that slot: it fills the slot and plays its source from IN, so a moved
/// marker rolls it and a moved end plays it on, as with a cover. An Off has no picture.
fn clip_covers(view: &SegmentsView, slots: &[Slot]) -> Vec<CoverSpan> {
    if !view.clips_are_slots {
        return Vec::new();
    }
    view.rows
        .iter()
        .filter(|row| row.kind.has_base_video())
        .filter_map(|row| {
            let slot = slots.iter().find(|slot| slot.start_frame == row.start_frame)?;
            Some(CoverSpan {
                cover_id: row.segment_id.clone(),
                slot_id: slot.slot_id.clone(),
                start_frame: row.start_frame,
                end_frame: row.end_frame,
                selected: false,
                clip_id: row.clip_id.clone(),
                source_in_frame: row.source_in_frame,
                source_out_frame: row.source_out_frame,
                a2_source_channel: 0,
                a2_level_db: 0,
                a2_off: true,
                level_keys: Vec::new(),
            })
        })
        .collect()
}

/// What a click in the zoom sends: the same action the timeline's mouse map gives that gesture
/// there (AGENTS 0.12: the zoom is the timeline enlarged), with its target and its frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StillAction {
    pub action: String,
    pub target: Option<String>,
    pub value: Option<i64>,
}

impl StillAction {
    pub fn new(action: &str, target: Option<String>, value: Option<u64>) -> Self {
        Self { action: action.to_string(), target, value: value.and_then(|value| i64::try_from(value).ok()) }
    }
}

/// The zoom of a story's Wrap rows as the timeline enlarged (AGENTS 0.12, user 2026-11-05: "the
/// same rule in the Story's and the S-Story's zoom, everywhere a timeline and its zoom are"):
/// under each picture the length strip of the slot it shows, a right click on the strip takes
/// that slot (as Ctrl+S, the timeline's right click on a strip), a right click on the picture
/// the marker of the cut (as on the timeline's filmstrip). A left click picks the side (the
/// zoom's one pick, never closing it).
pub fn story_zoom_clicks(stills: &mut [EditStill], view: &crate::SegmentsView) {
    for still in stills.iter_mut().filter(|still| still.elements.is_none() && !still.middle) {
        let frame = if still.after { still.marker_frame } else { still.marker_frame.saturating_sub(1) };
        if let Some(slot) = crate::slot_at(&view.slots, frame) {
            let frames = slot.end_frame.saturating_sub(slot.start_frame);
            let label = view.timebase.map_or_else(String::new, |(num, den)| crate::duration_label(frames, num, den));
            still.strip = Some(StillStrip {
                label,
                frames,
                fill: None,
                chosen: slot.selected,
                select: Some(StillAction::new("zoom_take_slot", Some(slot.slot_id.clone()), None)),
                take: StillAction::new("select_current_marker_slot", Some(slot.slot_id.clone()), None),
            });
        }
        // The taken marker (its draft frame may differ from the stored one), else the nearest.
        let usable = || view.markers.iter().filter(|marker| !marker.locked || view.clips_are_slots);
        let marker = usable().find(|marker| marker.taken).or_else(|| usable().min_by_key(|marker| marker.frame.abs_diff(still.marker_frame)));
        still.right_click = marker.map(|marker| StillAction::new("select_marker", Some(marker.marker_id.clone()), Some(frame)));
    }
}

#[cfg(test)]
mod zoom_click_tests {
    use super::*;
    use crate::{MarkerPin, SegmentsView, Slot, TakenPart};

    /// The Story's zoom is its Wrap rows enlarged (user 2026-11-05): each picture gets its slot's
    /// strip, a right click on it is Ctrl+S of that slot, a right click on the picture the taken
    /// marker; a left click is left to the zoom's one pick of the side (never a keep).
    #[test]
    fn a_story_zoom_picture_has_its_slots_strip_and_the_markers_right_click() {
        let slot = |id: &str, start, end| Slot { slot_id: id.into(), start_frame: start, end_frame: end, has_cover: false, selected: id == "s2" };
        let marker = |id: &str, frame, taken| MarkerPin { marker_id: id.into(), frame, locked: false, selected: false, taken, taken_part: TakenPart::Whole, origin: None, sound_tail: None, sound_lead: None, join: false, a1_tail: None, a1_lead: None, sound_lane: 0 };
        let view = SegmentsView { slots: vec![slot("s1", 0, 100), slot("s2", 100, 150)], markers: vec![marker("m1", 100, true), marker("m2", 150, false)], timebase: Some((25, 1)), ..Default::default() };
        let mut stills = vec![EditStill::new(100, false, None), EditStill::new(100, true, None)];
        story_zoom_clicks(&mut stills, &view);
        let strip = |index: usize| stills[index].strip.clone().unwrap();
        assert_eq!((strip(0).frames, strip(0).take.target.as_deref()), (100, Some("s1")), "the slot before the cut");
        assert_eq!((strip(1).frames, strip(1).take.target.as_deref(), strip(1).chosen), (50, Some("s2"), true), "the slot after it");
        assert_eq!(strip(0).take.action, "select_current_marker_slot", "a right click on a strip is the timeline's");
        assert_eq!(strip(0).select.as_ref().map(|action| action.action.as_str()), Some("zoom_take_slot"), "a left click on the strip is its slot");
        let right = |index: usize| stills[index].right_click.clone().map(|action| (action.action, action.target, action.value));
        assert_eq!(right(0), Some(("select_marker".into(), Some("m1".into()), Some(99))), "a right click on the picture: the marker");
        assert_eq!(right(1), Some(("select_marker".into(), Some("m1".into()), Some(100))));
    }
}

/// The pictures at the point of a taken level curve (docs/98 2.1, the zoom is the timeline
/// enlarged around the playhead): the last picture before the playhead line and the one on it.
pub fn point_stills(view: &SegmentsView, slots: &[Slot], frame: u64) -> Vec<EditStill> {
    crate::CutPictures {
        cut: frame,
        before: (frame > 0).then(|| (side_picture(view, slots, &|frame| frame, frame, false), qnc_ui_text::text("story.still.before_point"))),
        after: (frame < view.total_frames).then(|| (side_picture(view, slots, &|frame| frame, frame, true), qnc_ui_text::text("story.still.at_point"))),
        moving_after: true,
    }
    .stills()
}

/// The players of a zoom (docs/zoom-skeleton.md [B], user 2026-11-25): while a side is taken
/// (`cut`) and its owner gives where the edge was before it moved (`origin`), that side becomes the
/// player (the monitor's picture as the arrows and the wheel move the edge) and the middle place
/// holds the old picture for comparison. Nothing taken or no origin: the middle is the player of
/// the whole timeline. After Save the owner gives no origin and everything is back.
pub fn hand_player(stills: &mut [EditStill], origin: Option<StillPicture>) {
    let Some(origin) = origin else { return };
    let Some(side) = stills.iter().position(|still| still.cut && !still.middle) else { return };
    let Some(middle) = stills.iter().position(|still| still.middle) else { return };
    // Not moved (just taken, or saved): the middle stays the player of the whole timeline.
    if stills[side].picture.as_ref() == Some(&origin) {
        return;
    }
    stills[side].live = true;
    let held = &mut stills[middle];
    held.live = false;
    held.id = format!("qnc_edit_still:{}:{}", origin.clip_id, origin.source_frame);
    held.picture = Some(origin);
    held.label = qnc_ui_text::text("zoom.before").to_string();
}

#[cfg(test)]
mod player_tests {
    use super::*;

    fn three(cut_in: bool) -> Vec<EditStill> {
        let at = |frame| Some(StillPicture { clip_id: "c".into(), source_frame: frame });
        let mut in_still = EditStill::new(10, true, at(10));
        in_still.cut = cut_in;
        let mut player = EditStill::new(40, true, at(40));
        (player.live, player.middle) = (true, true);
        vec![in_still, player, EditStill::new(90, false, at(89))]
    }

    #[test]
    fn a_taken_side_is_the_player_and_the_middle_holds_where_it_was() {
        let mut stills = three(true);
        hand_player(&mut stills, Some(StillPicture { clip_id: "c".into(), source_frame: 7 }));
        assert!(stills[0].live, "the taken IN plays");
        assert!(!stills[1].live && stills[1].middle, "the middle no longer plays");
        assert_eq!(stills[1].picture.as_ref().map(|p| p.source_frame), Some(7), "it holds the old IN");
        assert!(!stills[2].live);
    }

    #[test]
    fn nothing_taken_or_no_origin_leaves_the_middle_the_player() {
        let mut stills = three(false);
        hand_player(&mut stills, Some(StillPicture { clip_id: "c".into(), source_frame: 7 }));
        assert!(stills[1].live && !stills[0].live);
        let mut stills = three(true);
        hand_player(&mut stills, None);
        assert!(stills[1].live && !stills[0].live);
        let mut stills = three(true);
        hand_player(&mut stills, Some(StillPicture { clip_id: "c".into(), source_frame: 10 }));
        assert!(stills[1].live && !stills[0].live, "a side where it was (saved) leaves the middle the player");
    }
}




#[cfg(test)]
mod clip_zoom_tests {
    use super::*;

    /// AGENTS 3a (user 2026-11-27: "maximalno 3 slike"): `[IN] [SLOT] [OUT]`; the neighbour's edge
    /// taken stands in its side's window, the M marker on that side, the slot the player.
    #[test]
    fn a_clips_zoom_is_at_most_three_windows_in_slot_out() {
        let action = |name: &str, target: &str| StillAction::new(name, Some(target.into()), None);
        let side = |take: &str, neighbour: Option<&str>, marker: Option<&str>| ClipZoomEdge {
            picture: None,
            frame: 0,
            take: action("take", take),
            neighbour: neighbour.map(|n| (None, action("take", n))),
            marker: marker.map(|m| action("marker", m)),
            sounds: [marker.map(|m| action("a1", m)), None],
        };
        let strip = StillStrip { label: String::new(), frames: 50, fill: None, chosen: false, select: None, take: action("x", "x") };
        let stills = clip_zoom_stills(side("in", Some("prev-out"), Some("m0")), side("out", Some("next-in"), None), strip.clone(), (5, None), ClipZoomTaken::Nothing);
        let takes: Vec<_> = stills.iter().map(|still| still.click.as_ref().and_then(|click| click.target.clone())).collect();
        assert_eq!(takes, [Some("in".into()), None, Some("out".into())], "IN, the slot, OUT");
        assert!(stills[1].live && stills[1].strip.is_some(), "the slot is the player with its strip");
        let marker = stills[0].elements.as_ref().and_then(|elements| elements.marker.as_ref()).and_then(|marker| marker.action.target.clone());
        assert_eq!(marker.as_deref(), Some("m0"), "the M marker on its side");
        let chosen = clip_zoom_stills(side("in", Some("prev-out"), Some("m0")), side("out", None, None), strip, (5, None), ClipZoomTaken::Neighbour { right: false });
        assert_eq!(chosen.len(), 3);
        assert_eq!(chosen[0].click.as_ref().and_then(|click| click.target.clone()).as_deref(), Some("prev-out"), "the OUT before in the left window");
        assert!(chosen[0].live && chosen[0].elements.as_ref().unwrap().picture, "the player there");
    }
}
