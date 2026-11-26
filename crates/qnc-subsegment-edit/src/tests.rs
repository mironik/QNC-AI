use super::*;

#[test]
fn a_part_slides_as_a_slot_its_neighbours_edges_or_pauses_give_way() {
    // Parts 0..50 of a, then b 100..150 straight after, then c 10..60 after a pause of 8.
    let mut pieces = vec![part("a", 0, 50, 0), part("b", 100, 150, 0), part("c", 10, 60, 8)];
    let length = |_: &str| 80;
    assert_eq!(crate::slide_range(&pieces, 1, length), Some((-49, 8)), "a keeps a frame and its clip; the pause after never goes below none");
    assert_eq!(crate::slide_range(&pieces, 0, length), None, "the first part does not move whole");
    assert_eq!(crate::slide_range(&pieces, 2, length), None, "nor the last");
    crate::slide_pieces(&mut pieces, 1, 5);
    let shown: Vec<_> = pieces.iter().map(|p| (p.from_frame, p.to_frame, p.gap_before)).collect();
    assert_eq!(shown, [(0, 55, 0), (100, 150, 0), (10, 60, 3)], "the part keeps its own range; a plays on, the pause after shrinks");
}

#[test]
fn the_sound_of_a_join_moves_as_a_storys_cut() {
    use crate::{JoinSide, JoinSound};
    let join = |lead, tail, side| JoinSound { index: 1, lead, tail, side, picture: false };
    let room = (-40, -40);
    // Both sides together: the sound cut (or the overlap between them) moves whole.
    assert_eq!(join(0, 0, JoinSide::Both).moved(3, room), join(-3, 3, JoinSide::Both), "the sound cut 3 frames after the picture cut");
    assert_eq!(join(4, 6, JoinSide::Both).moved(2, room), join(2, 8, JoinSide::Both));
    // One side alone, either way of the cut (user 2026-11-04): the first sound ending before the
    // cut and the second starting after it leave a hole between them.
    assert_eq!(join(0, 0, JoinSide::Tail).moved(-5, room), join(0, -5, JoinSide::Tail), "the sound before ends 5 early");
    assert_eq!(join(0, -5, JoinSide::Lead).moved(4, room), join(-4, -5, JoinSide::Lead), "the sound after starts 4 late");
    // Each stays inside its own part.
    assert_eq!(join(0, 0, JoinSide::Tail).moved(-100, room), join(0, -40, JoinSide::Tail));
}

/// The level curve of a part's A1 is a segment's (user 2026-11-07): taken on its part, the point
/// on the playhead line goes up a dB per key at the clip frame playing there, the draft is heard
/// and drawn, a click while it is taken keeps it and places the playhead.
#[test]
fn a_parts_level_curve_is_taken_and_raised_at_the_playhead_line() {
    let pieces = vec![part("a", 100, 150, 0), part("b", 10, 60, 0)];
    let block = Block::of_pieces("sot", pieces.clone());
    let mut edit = SubsegmentEdit::default();
    edit.open_subsegment = Some(block.id);
    edit.view.subsegment = Some(subsegment_view(&block, Some(60)));
    let preview = qnc_source_preview::SourcePreview::default();
    assert_eq!(edit.level_key("select_level_a1", Some("1"), &block, &pieces, &preview), Some(true));
    edit.level_key("step_prev_part", None, &block, &pieces, &preview);
    edit.level_key("step_prev_part", None, &block, &pieces, &preview);
    assert_eq!(edit.level_draft(), Some((1, vec![(20, 20)])), "part b plays its frame 20 at the playhead: +2 dB there");
    let shown = edit.view.subsegment.clone().unwrap();
    let curves = edit.level_curves(&shown, &pieces);
    assert_eq!(curves.iter().map(|curve| (curve.id.as_str(), curve.taken)).collect::<Vec<_>>(), vec![("0", false), ("1", true)], "every part has its line, the taken one as drafted");
    assert_eq!(edit.drafted(block.clone()).pieces[1].level_keys, vec![(20, 20)], "the draft is what plays");
    edit.value = Some(80);
    assert_eq!(edit.own("timeline_cue_frame", None), Some((crate::LEVEL_KEEP_AT.to_string(), Some("80".to_string()))), "a click keeps it and places the playhead");
    assert_eq!(edit.level_key("clear_focus", None, &block, &pieces, &preview), Some(true));
    assert!(!edit.level_taken(), "Esc gives it up");
}

/// AI on the SubSegment (user 2026-11-07): Shift+N turns it on and off for the open SubSegment;
/// a part with a curve of its own keeps it, the assistant's curves fill only the others; KEEP
/// without a proposal writes nothing.
#[test]
fn ai_is_the_open_subsegments_and_fills_only_parts_without_a_curve() {
    let mut pieces = vec![part("a", 100, 150, 0), part("b", 10, 60, 0)];
    pieces[0].level_keys = vec![(110, -30)];
    let block = Block::of_pieces("sot", pieces.clone());
    let preview = qnc_source_preview::SourcePreview::default();
    let mut edit = SubsegmentEdit::default();
    assert_eq!(edit.sound_assist_on(), None, "nothing open: the story's AI");
    edit.open_subsegment = Some(block.id);
    edit.view.subsegment = Some(subsegment_view(&block, Some(0)));
    assert_eq!(edit.assist_key("toggle_sound_assist", &block, &pieces, &preview), Some(true));
    assert_eq!(edit.sound_assist_on(), Some(true));
    edit.assist_curves = vec![(0, vec![(100, -50)]), (1, vec![(20, -40)])];
    let drafted = edit.drafted(block.clone());
    assert_eq!(drafted.pieces[0].level_keys, vec![(110, -30)], "the part's own curve stays");
    assert_eq!(drafted.pieces[1].level_keys, vec![(20, -40)], "the other plays the assistant's");
    edit.assist_curves.clear();
    assert_eq!(edit.assist_key("sound_assist_keep", &block, &pieces, &preview), Some(false), "no proposal: nothing kept");
    edit.assist_key("toggle_sound_assist", &block, &pieces, &preview);
    assert_eq!(edit.sound_assist_on(), Some(false));
}

fn part(clip: &str, from: u64, to: u64, gap: u64) -> Piece {
    Piece { clip_id: clip.into(), clip_name: clip.into(), from_frame: from, to_frame: to, start_tc: String::new(), end_tc: String::new(), length: String::new(), seconds: 1.0, text: String::new(), gap_before: gap, cut: false, a1_channel: None, a1_level_db: 0, a1_off: false, a1_lead_frames: 0, a1_tail_frames: 0, v_lead_frames: 0, v_tail_frames: 0, level_keys: Vec::new(), smooth_frames: 0, smooth_clip: String::new() }
}

#[test]
fn the_filmstrip_of_a_subsegment_lays_each_part_after_its_pause() {
    let pieces = vec![part("a", 100, 150, 0), part("b", 10, 60, 5)];
    let shown = subsegment_view(&Block::of_pieces("sot", pieces.clone()), None);
    let at: Vec<_> = subsegment_pictures(&shown, &pieces).iter().map(|p| (p.clip_id.clone(), p.program_start, p.program_end, p.source_in, p.source_out)).collect();
    assert_eq!(at, vec![("a".to_string(), 0, 50, 100, 150), ("b".to_string(), 55, 105, 10, 60)]);
}

#[test]
fn the_zoom_of_a_subsegment_edge_shows_the_pictures_either_side_of_its_cut() {
    let pieces = vec![part("a", 100, 150, 0), part("a", 300, 340, 0), part("b", 10, 60, 5)];
    let block = Block::of_pieces("sot", pieces.clone());
    let shown = subsegment_view(&block, None);
    assert_eq!(shown.parts.iter().map(|p| (p.start, p.end)).collect::<Vec<_>>(), vec![(0, 50), (50, 90), (95, 145)]);
    // The IN of the second part: the last picture of the first part, then its own first, marked.
    let zoom = subsegment_zoom(&shown, &pieces, (1, false), (50, None), |_| Some(1000)).unwrap();
    let pictures: Vec<_> = zoom.source_edit_stills.iter().filter(|s| !s.middle).cloned().collect::<Vec<_>>().iter().map(|s| (s.picture.as_ref().map(|p| (p.clip_id.clone(), p.source_frame)), s.cut)).collect::<Vec<_>>();
    assert_eq!(pictures, vec![(Some(("a".to_string(), 149)), false), (Some(("a".to_string(), 300)), true)]);
    // The player stands in the middle, between the two sides (user 2026-11-23).
    assert!(zoom.source_edit_stills.get(1).is_some_and(|still| still.middle), "the player in the middle");
    let sound = zoom.zoom_sound.unwrap();
    assert_eq!((sound.from, sound.to, sound.cut), (0, 100, 50));
    assert_eq!(sound.lanes[0].len(), 3, "every part within a second of the cut is heard on A1");
    // The zoom is the timeline enlarged (AGENTS 0.12, user 2026-11-05): every click on a picture
    // or its strip sends the timeline's action of the same gesture at the picture's frame, so the
    // owner answers the same as for the timeline. Left of the cut part 0 (its last frame 49),
    // right of it part 1 (its first frame 50).
    let own = |action: &qnc_program_view::StillAction| crate::own_action(&shown, &action.action, action.value, action.target.as_deref(), false);
    let at = |still: usize| zoom.source_edit_stills.iter().filter(|s| !s.middle).cloned().collect::<Vec<_>>()[still].clone();
    let strip = |still: usize| at(still).strip.unwrap();
    assert_eq!((strip(0).frames, strip(1).frames), (50, 40), "each picture's slot length strip");
    let some = |action: &str, target: &str| Some((action.to_string(), Some(target.to_string())));
    // A left click anywhere on a strip only chooses that slot (user 2026-11-26).
    let pick = |still: usize| strip(still).select.map(|select| (select.action, select.target));
    assert_eq!(pick(0), Some((crate::SUBSEGMENT_SELECT.to_string(), Some("0".to_string()))));
    assert_eq!(pick(1), Some((crate::SUBSEGMENT_SELECT.to_string(), Some("1".to_string()))));
    assert_eq!(own(&strip(0).take), some(crate::TAKE_EDGE, "0:out:slot"), "a right click on the strip: the slot, its edge at the cut");
    assert_eq!(own(&strip(1).take), some(crate::TAKE_EDGE, "1:in:slot"));
    assert_eq!(own(at(0).right_click.as_ref().unwrap()), some(crate::TAKE_EDGE, "0:out"), "a right click on the picture: its own marker there");
    assert_eq!(own(at(1).right_click.as_ref().unwrap()), some(crate::TAKE_EDGE, "1:in"));
    // The IN of the third part follows a pause: on the left is still the OUT of the part before.
    let zoom = subsegment_zoom(&shown, &pieces, (2, false), (50, None), |_| Some(1000)).unwrap();
    assert_eq!(zoom.source_edit_stills.iter().filter(|s| !s.middle).cloned().collect::<Vec<_>>()[0].picture.as_ref().map(|p| (p.clip_id.as_str(), p.source_frame)), Some(("a", 339)));
    // The OUT of the first part: its last picture, marked, then the next part's first.
    let zoom = subsegment_zoom(&shown, &pieces, (0, true), (50, None), |_| Some(1000)).unwrap();
    let pictures: Vec<_> = zoom.source_edit_stills.iter().filter(|s| !s.middle).cloned().collect::<Vec<_>>().iter().map(|s| (s.picture.as_ref().map(|p| p.source_frame), s.cut)).collect();
    assert_eq!(pictures, vec![(Some(149), true), (Some(300), false)]);
    // The first part's IN has no OUT before it; the last part's OUT no IN after it.
    let first = subsegment_zoom(&shown, &pieces, (0, false), (50, None), |_| Some(1000)).unwrap();
    assert!(first.source_edit_stills.iter().filter(|s| !s.middle).cloned().collect::<Vec<_>>()[0].strip.is_none(), "nothing before the first IN to pick");
    let last = subsegment_zoom(&shown, &pieces, (2, true), (50, None), |_| Some(1000)).unwrap();
    assert!(last.source_edit_stills.iter().filter(|s| !s.middle).cloned().collect::<Vec<_>>()[1].strip.is_none());
}

#[test]
fn the_zoom_of_a_scope_shows_its_in_the_player_and_its_out_and_a_picture_takes_that_marker() {
    // A margin before, two slots, a margin after (user 2026-11-23: a slot is between two markers).
    let pieces = vec![part("a", 80, 100, 0), part("a", 100, 150, 0), part("b", 10, 60, 0), part("b", 60, 70, 0)];
    let shown = subsegment_view(&Block::of_pieces("sot", pieces.clone()), None);
    let sides = |zoom: &qnc_program_view::SegmentsView| zoom.source_edit_stills.iter().map(|s| (s.live, s.picture.as_ref().map(|p| (p.clip_id.clone(), p.source_frame)), s.click.as_ref().map(|c| (c.action.clone(), c.target.clone())))).collect::<Vec<_>>();
    // The whole statement: the first slot's IN, the player, the last slot's OUT.
    let whole = scope_zoom(&shown, &pieces, (1, 2), None, (50, None), |_| Some(1000)).unwrap();
    let at = sides(&whole);
    assert_eq!(at[0], (false, Some(("a".into(), 100)), Some((crate::TAKE_EDGE.to_string(), Some("1:in".to_string())))));
    assert_eq!(at[1].2, Some((crate::SUBSEGMENT_SELECT.to_string(), Some("whole".to_string()))), "the player shows the whole statement");
    assert_eq!(at[2], (false, Some(("b".into(), 59)), Some((crate::TAKE_EDGE.to_string(), Some("2:out".to_string())))));
    // One slot: its own IN and OUT, its OUT taken framed.
    let slot = scope_zoom(&shown, &pieces, (1, 1), Some(true), (50, None), |_| Some(1000)).unwrap();
    let at = sides(&slot);
    assert_eq!((at[0].1.clone(), at[2].1.clone()), (Some(("a".into(), 100)), Some(("a".into(), 149))));
    assert!(slot.source_edit_stills[2].cut && !slot.source_edit_stills[0].cut);
    assert_eq!(slot.source_edit_stills[0].strip.as_ref().and_then(|s| s.select.as_ref()).and_then(|s| s.target.clone()), Some("1".to_string()));
}

/// AGENTS 3a (user 2026-11-26): a part's zoom is IN, the slot (the player with its one strip),
/// OUT; an M marker (that OUT/IN) on the outer edge where it meets a neighbour without a pause;
/// IN or OUT changes its length alone, an M both parts'.
#[test]
fn a_parts_zoom_is_in_slot_out_with_its_m_markers_where_it_meets_a_neighbour() {
    use qnc_program_view::ClipZoomTaken;
    let pieces = [part("a", 0, 50, 0), part("b", 10, 50, 0), part("c", 0, 50, 5)];
    let span = |start: u64, end: u64| crate::SubsegmentPart { start, end, clip_name: String::new(), length: String::new(), gap_before: 0, cut: false, a1_channel: None, a1_level_db: 0, a1_off: false, channels: None, a1_lead: 0, a1_tail: 0, smooth_frames: 0, smooth_ready: None };
    let view = crate::SubsegmentView { parts: vec![span(0, 50), span(50, 90), span(95, 145)], total_frames: 145, ..Default::default() };
    let zoom = crate::view::clip_zoom(&view, &pieces, 1, ClipZoomTaken::Nothing, (25, None), |_| None).unwrap();
    let stills = &zoom.source_edit_stills;
    let target = |index: usize| stills[index].click.as_ref().and_then(|action| action.target.clone());
    assert_eq!((target(0).as_deref(), target(2).as_deref()), (Some("edge:0:1"), Some("edge:1:1")), "IN and OUT of this part");
    let marker = |index: usize| stills[index].elements.as_ref().and_then(|elements| elements.marker.as_ref()).and_then(|marker| marker.action.target.clone());
    assert_eq!(marker(0).as_deref(), Some("marker:0:1"), "an M where it meets the part before");
    assert_eq!(marker(2), None, "a pause after it: no M");
    assert!(stills[1].live && stills[1].strip.is_some(), "the slot is the player with its strip");
    let chosen = crate::view::clip_zoom(&view, &pieces, 1, ClipZoomTaken::Marker { right: false }, (25, None), |_| None).unwrap();
    assert!(chosen.source_edit_stills[0].live, "the M with the part before chosen: the player in the IN window");
}
