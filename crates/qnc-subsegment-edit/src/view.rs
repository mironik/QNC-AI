//! What the open SubSegment shows: its parts on its own axis (a pause before a part moves it
//! on), the A1 wave and filmstrip placed on that axis, and the trim zoom of a taken edge. No
//! state, no storage.

use qnc_script_db::{Block, Piece};
pub use qnc_script_draft::{block_pieces, piece_inputs};


/// One part of the SubSegment open on the source timeline's place: its frames on the
/// SubSegment's axis, its clip and length, and the pause and join before it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsegmentPart {
    pub start: u64,
    pub end: u64,
    pub clip_name: String,
    pub length: String,
    pub gap_before: u64,
    pub cut: bool,
    /// Its own A1: the camera channel (`None`: the one kept for its clip), level in dB, off.
    pub a1_channel: Option<u16>,
    pub a1_level_db: i8,
    pub a1_off: bool,
    /// How many channels its clip has (from its stored wave), once read.
    pub channels: Option<u16>,
    /// The J and L of the join before it (frames), as drafted.
    pub a1_lead: i64,
    pub a1_tail: i64,
    /// The pictures the smooth cut of the join before it replaces (0: none) and its state:
    /// `Some(true)` playing, `Some(false)` being made, `None` it cannot be at this join.
    pub smooth_frames: u64,
    pub smooth_ready: Option<bool>,
}

/// The SubSegment open in place of the source timeline (user 2026-10-29): its parts one after
/// another on V1 and A1 as the player plays them, the playhead of that player.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SubsegmentView {
    pub block_id: i64,
    pub parts: Vec<SubsegmentPart>,
    pub total_frames: u64,
    pub playhead: Option<u64>,
    /// The A1 wave of the parts on the SubSegment's axis (peaks over the whole of it).
    pub a1: Vec<f32>,
    /// The filmstrip of the parts on the SubSegment's axis (user 2026-10-29: as a Wrap row
    /// shows its segments), from `qnc-program-filmstrip`.
    pub pictures: Vec<qnc_program_filmstrip::ProgramPicture>,
    /// The edge being trimmed: its part, whether it is the OUT, and its frame on the axis.
    pub trim: Option<(usize, bool, u64)>,
    /// The trim zoom of the taken edge (user 2026-10-29: as a cover's): the pictures either
    /// side of the cut, the fine A1 wave and the words around it.
    pub zoom: Option<Box<qnc_program_view::SegmentsView>>,
    /// The part whose A1 is taken (Ctrl+1), as drafted.
    pub sound: Option<usize>,
    /// The channel column and level of the taken A1 (as the source timeline shows its lane,
    /// AGENTS 0.12): drafted, open while it is taken.
    pub sound_choice: Option<qnc_lane_choice::ChannelChoice>,
    /// The frame rate of the parts (of the first part's clip), once known: a part's length in
    /// seconds and its colour as a cover's (user 2026-11-03).
    pub rate: Option<(u32, u32)>,
    /// The taken edge is the whole cut of a join: the trimmed part's OUT and the next part's IN.
    pub roll: bool,
    /// The part chosen by a click on its length strip, as a slot is.
    pub chosen: Option<usize>,
    /// Every chosen slot (several after a drag over their strips) and every slot whose A1 is
    /// changed with the taken one (user 2026-11-07).
    pub chosen_slots: Vec<usize>,
    /// The level curves of the parts' A1 on the axis (the taken one as drafted; user 2026-11-07,
    /// as a segment's on a Wrap row).
    pub level_curves: Vec<qnc_program_view::LevelCurve>,
    /// The sound of the join before this part taken (Ctrl+1 on a taken cut) and which side moves.
    pub join_sound: Option<(usize, crate::JoinSide)>,
    /// The part taken whole as a slot (Ctrl+S), moved as drafted.
    pub slide: Option<usize>,
    /// The narrow video row instead of the filmstrip (Ctrl+click on V, as on a Wrap row).
    pub video_compact: bool,
    /// The window of the part enlarged by a double click (70/15/15, as a story's slot).
    pub enlarged: Option<(u64, u64)>,
    /// The clip shown before the first part and after the last (frames), as a statement's
    /// window (user 2026-11-23: 1/13 either side, as on Izjave); not parts, shown dimmed.
    pub margins: (u64, u64),
}

/// The parts on the SubSegment's axis (a pause before a part moves it on).
pub fn subsegment_view(block: &Block, playhead: Option<u64>) -> SubsegmentView {
    let mut start = 0;
    let pieces = block_pieces(block);
    let smooth: Vec<Option<bool>> = (0..pieces.len()).map(|index| index.checked_sub(1).and_then(|before| qnc_script_draft::smooth_of(&pieces[before], &pieces[index])).map(|(id, _)| id == pieces[index].smooth_clip)).collect();
    let parts = pieces
        .into_iter()
        .zip(smooth)
        .enumerate()
        .map(|(index, (piece, smooth_ready))| {
            let gap = if index == 0 { 0 } else { piece.gap_before };
            start += gap;
            let end = start + piece.to_frame.saturating_sub(piece.from_frame);
            let part = SubsegmentPart { start, end, clip_name: piece.clip_name, length: piece.length, gap_before: gap, cut: piece.cut, a1_channel: piece.a1_channel, a1_level_db: piece.a1_level_db, a1_off: piece.a1_off, channels: None, a1_lead: if index == 0 || gap > 0 { 0 } else { piece.a1_lead_frames }, a1_tail: if index == 0 || gap > 0 { 0 } else { piece.a1_tail_frames }, smooth_frames: if index == 0 { 0 } else { piece.smooth_frames }, smooth_ready };
            start = end;
            part
        })
        .collect();
    SubsegmentView { block_id: block.id, parts, total_frames: start, playhead, a1: Vec::new(), pictures: Vec::new(), trim: None, zoom: None, sound: None, sound_choice: None, rate: None, roll: false, chosen: None, chosen_slots: Vec::new(), level_curves: Vec::new(), join_sound: None, slide: None, video_compact: false, enlarged: None, margins: (0, 0) }
}

/// Where the parts play on the SubSegment's axis, for its wave (channel 1 of each clip: the
/// wave of A1 as recorded).
pub fn subsegment_placements(view: &SubsegmentView, pieces: &[Piece]) -> Vec<qnc_program_waveform::Placement> {
    view.parts
        .iter()
        .zip(pieces)
        .map(|(part, piece)| qnc_program_waveform::Placement {
            clip_id: piece.clip_id.clone(),
            channel: piece.a1_channel.unwrap_or(0),
            program_start: part.start,
            program_end: part.end,
            source_in: piece.from_frame,
            source_out: piece.to_frame,
            level_db: piece.a1_level_db,
            off: piece.a1_off,
            // The wave follows the part's level curve, as a segment's does.
            curve: piece.level_keys.clone(),
        })
        .collect()
}

/// Where the parts' pictures are on the SubSegment's axis, for its filmstrip.
pub fn subsegment_pictures(view: &SubsegmentView, pieces: &[Piece]) -> Vec<qnc_program_filmstrip::PicturePlacement> {
    view.parts
        .iter()
        .zip(pieces)
        .map(|(part, piece)| qnc_program_filmstrip::PicturePlacement {
            clip_id: piece.clip_id.clone(),
            program_start: part.start,
            program_end: part.end,
            source_in: piece.from_frame,
            source_out: piece.to_frame,
        })
        .collect()
}

/// The length strip of part `index` under a zoom picture standing at `frame` of the axis: a left
/// click chooses that slot, a right click takes it, as on the timeline.
fn part_strip(view: &SubsegmentView, index: usize, frame: u64) -> Option<qnc_program_view::StillStrip> {
    let part = view.parts.get(index)?;
    let frames = part.end.saturating_sub(part.start);
    let label = view.rate.map_or_else(|| part.length.clone(), |(num, den)| qnc_program_view::duration_label(frames, num, den));
    Some(qnc_program_view::StillStrip {
        label,
        frames,
        fill: None,
        chosen: view.chosen == Some(index),
        select: Some(qnc_program_view::StillAction::new(crate::SUBSEGMENT_SELECT, Some(index.to_string()), None)),
        take: qnc_program_view::StillAction::new("timeline_take_cover", Some(format!("part-{index}")), Some(frame)),
    })
}

/// The zoom of a scope (user 2026-11-23): parts `from` to `to` (the whole statement or one slot;
/// a slot is the space between two markers). On the left the scope's IN (its first part's first
/// picture), in the middle the player, on the right its OUT (its last part's last picture). A
/// click on the player shows the whole statement, on a strip that slot, on a picture takes that
/// marker (left IN, right OUT); `taken` frames the edge held (false IN, true OUT).
pub fn scope_zoom(
    view: &SubsegmentView,
    pieces: &[Piece],
    (from, to): (usize, usize),
    taken: Option<bool>,
    (rate, follow): (u64, Option<(&qnc_program_view::ZoomFollow, Option<(u64, bool)>)>),
    clip_frames: impl Fn(&str) -> Option<u64>,
) -> Option<qnc_program_view::SegmentsView> {
    use qnc_program_view::{EditStill, StillAction, StillPicture};
    let (first, last) = (view.parts.get(from)?, view.parts.get(to)?);
    let (first_piece, last_piece) = (pieces.get(from)?, pieces.get(to)?);
    let picture = |piece: &Piece, frame: u64| Some(StillPicture { clip_id: piece.clip_id.clone(), source_frame: frame });
    let (in_frame, out_frame) = (first.start, last.end.saturating_sub(1).max(last.start));
    let mut left = EditStill::new(in_frame, false, picture(first_piece, first_piece.from_frame)).labelled(&qnc_ui_text::text("story.still.in_first"));
    left.cut = taken == Some(false);
    left.click = Some(StillAction::new(crate::TAKE_EDGE, Some(format!("{from}:in")), Some(in_frame)));
    left.strip = part_strip(view, from, in_frame);
    left.clip_frames = clip_frames(&first_piece.clip_id);
    let mut right = EditStill::new(out_frame, true, picture(last_piece, last_piece.to_frame.saturating_sub(1).max(last_piece.from_frame))).labelled(&qnc_ui_text::text("story.still.out_last"));
    right.cut = taken == Some(true);
    right.click = Some(StillAction::new(crate::TAKE_EDGE, Some(format!("{to}:out")), Some(out_frame)));
    right.strip = part_strip(view, to, out_frame);
    right.clip_frames = clip_frames(&last_piece.clip_id);
    // The zoom's sound is around the edge taken, else around the scope's IN.
    let cut = if taken == Some(true) { last.end } else { first.start };
    let idle = qnc_program_view::ZoomFollow::default();
    let (follow, position) = follow.unwrap_or((&idle, None));
    let playhead = position.map_or(cut, |(frame, _)| frame.min(view.total_frames.saturating_sub(1)));
    let at_axis = view.parts.iter().zip(pieces).find(|(part, _)| (part.start..part.end).contains(&playhead)).and_then(|(part, piece)| picture(piece, piece.from_frame + (playhead - part.start)));
    let mut player = EditStill::new(playhead, true, at_axis).labelled(&qnc_ui_text::text("zoom.player"));
    player.live = true;
    player.middle = true;
    // The player clicked: the whole statement.
    player.click = Some(StillAction::new(crate::SUBSEGMENT_SELECT, Some("whole".to_string()), None));
    player.clip_frames = player.picture.as_ref().and_then(|picture| clip_frames(&picture.clip_id));
    let axis = qnc_program_view::ZoomAxis { cut, holds: (cut, cut), half: rate.max(1), total: view.total_frames, position, follow, lanes: [subsegment_placements(view, pieces), Vec::new()] };
    Some(qnc_program_view::SegmentsView { source_edit_stills: vec![left, player, right], zoom_sound: Some(axis.sound()), ..Default::default() })
}

/// The zoom of part `index` of a composite statement (AGENTS 3a "Model reza i zoom", user
/// 2026-11-26): a part is IN - SLOT - OUT. Its IN picture, the slot (the player with its one length
/// strip), its OUT picture; where it meets a neighbour without a pause the M marker (that OUT/IN) on
/// the outer edge. IN or OUT changes this part's length alone, an M both parts' (roll); the A1
/// under IN and OUT is that M marker's sound (the join's J/L). Every click is `ZOOM_ELEMENT` with
/// "kind:side:index".
pub fn clip_zoom(
    view: &SubsegmentView,
    pieces: &[Piece],
    index: usize,
    taken: qnc_program_view::ClipZoomTaken,
    (rate, follow): (u64, Option<(&qnc_program_view::ZoomFollow, Option<(u64, bool)>)>),
    clip_frames: impl Fn(&str) -> Option<u64>,
) -> Option<qnc_program_view::SegmentsView> {
    use qnc_program_view::{ClipZoomEdge, StillAction, StillPicture};
    let (part, piece) = (view.parts.get(index)?, pieces.get(index)?);
    let picture = |piece: &Piece, frame: u64| Some(StillPicture { clip_id: piece.clip_id.clone(), source_frame: frame });
    // Parts `c - 1` and `c` meet without a pause.
    let joined = |c: usize| c >= 1 && c < pieces.len() && pieces[c].gap_before == 0;
    let action_of = |kind: &str, right: bool, of: usize| StillAction::new(crate::ZOOM_ELEMENT, Some(format!("{kind}:{}:{of}:{index}", u8::from(right))), None);
    let action = |kind: &str, right: bool| action_of(kind, right, index);
    let edge = |right: bool| {
        let join = if right { joined(index + 1) } else { joined(index) };
        ClipZoomEdge {
            picture: if right { picture(piece, piece.to_frame.saturating_sub(1).max(piece.from_frame)) } else { picture(piece, piece.from_frame) },
            frame: if right { part.end } else { part.start },
            take: action("edge", right),
            // The neighbour there: the OUT of the part before, the IN of the part after.
            neighbour: if right { pieces.get(index + 1).map(|next| (picture(next, next.from_frame), action_of("edge", false, index + 1))) } else { index.checked_sub(1).and_then(|before| pieces.get(before)).map(|prev| (picture(prev, prev.to_frame.saturating_sub(1).max(prev.from_frame)), action_of("edge", true, index - 1))) },
            marker: join.then(|| action("marker", right)),
            sounds: [join.then(|| action("a1", right)), None],
        }
    };
    let frames = part.end.saturating_sub(part.start);
    let label = view.rate.map_or_else(|| part.length.clone(), |(num, den)| qnc_program_view::duration_label(frames, num, den));
    let strip = qnc_program_view::StillStrip { label, frames, fill: None, chosen: false, select: None, take: action("edge", false) };
    let idle = qnc_program_view::ZoomFollow::default();
    let (follow, position) = follow.unwrap_or((&idle, None));
    let playhead = position.map_or(part.start, |(frame, _)| frame.min(view.total_frames.saturating_sub(1)));
    let at_axis = view.parts.iter().zip(pieces).find(|(part, _)| (part.start..part.end).contains(&playhead)).and_then(|(part, piece)| picture(piece, piece.from_frame + (playhead - part.start)));
    let mut stills = qnc_program_view::clip_zoom_stills(edge(false), edge(true), strip, (playhead, at_axis), taken);
    for still in &mut stills {
        still.clip_frames = still.picture.as_ref().and_then(|picture| clip_frames(&picture.clip_id));
    }
    // The slot clicked (the player or its strip): the player again (user 2026-11-26).
    let player = StillAction::new(crate::ZOOM_PLAYER, Some(index.to_string()), None);
    if let Some(slot) = stills.iter_mut().find(|still| still.middle) {
        slot.click = Some(player.clone());
        if let Some(strip) = slot.strip.as_mut() {
            strip.select = Some(player);
        }
    }
    // The sound around the edge chosen, else around the playhead in the part.
    use qnc_program_view::ClipZoomTaken as T;
    let cut = match taken {
        T::Edge { right } | T::Neighbour { right } | T::Marker { right } | T::Sound { right, .. } => if right { part.end } else { part.start },
        T::Nothing => playhead,
    };
    let axis = qnc_program_view::ZoomAxis { cut, holds: (cut, cut), half: rate.max(1), total: view.total_frames, position, follow, lanes: [subsegment_placements(view, pieces), Vec::new()] };
    Some(qnc_program_view::SegmentsView { source_edit_stills: stills, zoom_sound: Some(axis.sound()), ..Default::default() })
}
