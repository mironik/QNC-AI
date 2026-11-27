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

/// AGENTS 3a (user 2026-11-27: "maximalno 3 slike"): a part's zoom is `[IN] [SLOT] [OUT]`; the M
/// marker (that OUT/IN pair) on the side where it meets a neighbour without a pause; a neighbour's
/// edge taken stands in its side's window and keeps this part shown.
#[test]
fn a_parts_zoom_is_in_slot_out_with_its_m_markers_where_it_meets_a_neighbour() {
    use qnc_program_view::ClipZoomTaken;
    let pieces = [part("a", 0, 50, 0), part("b", 10, 50, 0), part("c", 0, 50, 5)];
    let span = |start: u64, end: u64| crate::SubsegmentPart { start, end, clip_name: String::new(), length: String::new(), gap_before: 0, cut: false, a1_channel: None, a1_level_db: 0, a1_off: false, channels: None, a1_lead: 0, a1_tail: 0, smooth_frames: 0, smooth_ready: None };
    let view = crate::SubsegmentView { parts: vec![span(0, 50), span(50, 90), span(95, 145)], total_frames: 145, ..Default::default() };
    let zoom = crate::view::clip_zoom(&view, &pieces, 1, ClipZoomTaken::Nothing, (25, None), |_| None).unwrap();
    let stills = &zoom.source_edit_stills;
    let target = |index: usize| stills[index].click.as_ref().and_then(|action| action.target.clone());
    let targets: Vec<_> = (0..stills.len()).map(target).collect();
    assert_eq!(targets, [Some("edge:0:1:1".into()), Some("1".into()), Some("edge:1:1:1".into())], "IN, the slot, OUT");
    let marker = |index: usize| stills[index].elements.as_ref().and_then(|elements| elements.marker.as_ref()).and_then(|marker| marker.action.target.clone());
    assert_eq!(marker(0).as_deref(), Some("marker:0:1:1"), "the M with the part before on the IN side");
    assert_eq!(marker(2), None, "a pause after it: no M");
    assert!(stills[1].live && stills[1].strip.is_some(), "the slot is the player with its strip");
    let chosen = crate::view::clip_zoom(&view, &pieces, 1, ClipZoomTaken::Neighbour { right: false }, (25, None), |_| None).unwrap();
    assert_eq!(chosen.source_edit_stills.len(), 3);
    assert!(chosen.source_edit_stills[0].live, "the OUT of the part before chosen: the player in its window");
    assert_eq!(chosen.source_edit_stills[0].click.as_ref().and_then(|action| action.target.clone()).as_deref(), Some("edge:1:0:1"));
}
