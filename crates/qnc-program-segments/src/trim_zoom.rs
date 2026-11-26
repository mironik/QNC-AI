//! The trim zoom of a cut (AGENTS 3a "Model reza i zoom", user 2026-11-26): the left clip with its
//! slot, the player, the right clip with its slot. Each clip's strip is the ripple of its own slot
//! (the other keeps its length), its outer M the roll of its cut with the clip beyond it; the M
//! of the cut between them is under the player; its picture and A1/A2 the roll of that track.
//! Which cut and what is chosen follow from what is taken.

use super::*;
use qnc_program_view::{CutZoomClip, CutZoomCut, CutZoomTaken, EditStill, StillAction, StillStrip};

impl ProgramSegments {
    /// The cut a right click inside slot `slot_id` opens (user 2026-11-26): its start in its left
    /// half, its end in its right half.
    pub(crate) fn cut_of_slot(&self, slot_id: &str, frame: Option<u64>) -> Option<String> {
        let slot = self.stored_slots.iter().find(|slot| slot.slot_id == slot_id)?;
        let middle = (slot.start_frame + slot.end_frame) / 2;
        Some(if frame.is_some_and(|frame| frame >= middle) { slot.end_marker_id.clone() } else { slot.start_marker_id.clone() })
    }

    /// An element of the zoom of a cut clicked: one click, one action (AGENTS 3a).
    pub(crate) fn zoom_cut_element(&mut self, kind: &str, right: bool, id: &str) -> bool {
        self.zoom_pick = right;
        self.let_go_if_unmoved();
        match kind {
            // The ripple of that clip's slot alone: the left clip's end, the right clip's start.
            "strip" => {
                self.selected_slot = Some(id.to_string());
                self.take_slot_edge(right);
            }
            "outer" | "cut" => {
                self.select_marker(id);
                self.edit_marker();
            }
            "picture" => {
                self.take_cut_part(Some(id.to_string()), true, 0);
            }
            "a1" | "a2" => {
                self.take_cut_part(Some(id.to_string()), false, u8::from(kind == "a2"));
            }
            _ => return false,
        }
        true
    }

    /// The zoom of the cut, when one is open or something at it is taken; `None`: the zoom of
    /// other edits (a slot moved whole, a level line, a cut of two clips under no M marker).
    pub(crate) fn slot_zoom(&self, view: &SegmentsView, cuts: &[EditStill]) -> Option<Vec<EditStill>> {
        if self.slot_move.is_some() || self.level_line.is_some() {
            return None;
        }
        let stored = &self.stored_slots;
        let left_of = |marker: &str| stored.iter().find(|slot| slot.end_marker_id == marker);
        let right_of = |marker: &str| stored.iter().find(|slot| slot.start_marker_id == marker);
        let (cut, taken) = if let Some(trim) = &self.slot_trim {
            let slot = stored.iter().find(|slot| slot.slot_id == trim.slot_id)?;
            (if trim.start { slot.start_marker_id.clone() } else { slot.end_marker_id.clone() }, CutZoomTaken::Strip { right: trim.start })
        } else if let Some((marker_id, _)) = &self.marker_edit {
            // An outer M of the cut shown stays in that zoom; else the zoom is of the marker taken.
            let outer = self.zoom_cut.as_ref().and_then(|cut| {
                if left_of(cut).is_some_and(|slot| slot.start_marker_id == *marker_id) {
                    Some((cut.clone(), CutZoomTaken::OuterMarker { right: false }))
                } else if right_of(cut).is_some_and(|slot| slot.end_marker_id == *marker_id) {
                    Some((cut.clone(), CutZoomTaken::OuterMarker { right: true }))
                } else {
                    None
                }
            });
            outer.unwrap_or_else(|| {
                let taken = match self.cut_part {
                    cut_part::CutPart::Whole => CutZoomTaken::Cut,
                    cut_part::CutPart::Picture => CutZoomTaken::Picture { right: self.zoom_pick },
                    cut_part::CutPart::Sound { lane, .. } => CutZoomTaken::Sound { right: self.zoom_pick, lane },
                };
                (marker_id.clone(), taken)
            })
        } else {
            (self.zoom_cut.clone()?, CutZoomTaken::Nothing)
        };
        let (left, right) = (left_of(&cut), right_of(&cut));
        if left.is_none() && right.is_none() {
            return None;
        }
        let at = self.stored_markers.iter().find(|marker| marker.marker_id == cut).map(|marker| marker.program_frame)?;
        // The locked start and end of a story have no M marker to roll (an S-Story's do).
        let usable = |marker_id: &str| {
            let locked = view.markers.iter().any(|marker| marker.marker_id == marker_id && marker.locked);
            (!locked || view.clips_are_slots).then(|| marker_id.to_string())
        };
        let action = |kind: &str, right: bool, id: &str| StillAction::new("zoom_cut_element", Some(format!("{kind}:{}:{id}", u8::from(right))), None);
        let strip = |slot: &qnc_program_db::ProgramSlot, right: bool| {
            let frames = slot.end_frame.saturating_sub(slot.start_frame);
            let label = view.timebase.map_or_else(String::new, |(num, den)| qnc_program_view::duration_label(frames, num, den));
            let ripple = action("strip", right, &slot.slot_id);
            StillStrip { label, frames, fill: None, chosen: false, select: Some(ripple.clone()), take: ripple }
        };
        // The picture of each clip at the cut: as drafted where the edit moved it.
        let drafted = |right: bool| cuts.iter().find(|still| still.after == right).and_then(|still| still.picture.clone());
        let picture = |right: bool| if taken == CutZoomTaken::Nothing { None } else { drafted(right) }.or_else(|| qnc_program_view::program_picture(view, if right { at } else { at.saturating_sub(1) }));
        let clip = |slot: Option<&qnc_program_db::ProgramSlot>, right: bool| {
            slot.map(|slot| CutZoomClip {
                picture: picture(right),
                frame: at,
                strip: strip(slot, right),
                outer_marker: usable(if right { &slot.end_marker_id } else { &slot.start_marker_id }).map(|marker| action("outer", right, &marker)),
            })
        };
        let cut_element = usable(&cut).map(|marker| CutZoomCut {
            marker: action("cut", false, &marker),
            picture: [action("picture", false, &marker), action("picture", true, &marker)],
            sounds: [false, true].map(|right| [Some(action("a1", right, &marker)), Some(action("a2", right, &marker))]),
        });
        let playhead = view.playhead.unwrap_or(at);
        Some(qnc_program_view::cut_zoom_stills(clip(left, false), clip(right, true), cut_element, (playhead, qnc_program_view::program_picture(view, playhead)), taken))
    }
}
