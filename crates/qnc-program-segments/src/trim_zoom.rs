//! The trim zoom of a slot (AGENTS 3a "Model reza i zoom", user 2026-11-26): its IN picture, the
//! player and its OUT picture, with the strip, A1, A2 and the M marker on the edge pictures. Which
//! slot and what is chosen follow from what is taken: a slot edge (the strip, ripple), a marker
//! whole (the M bar, roll), its picture or its sound (roll of that track).

use super::*;
use qnc_program_view::{EditStill, SlotZoomTaken};

impl ProgramSegments {
    /// The zoom of the slot, when one is open or something at its edges is taken; `None`: the
    /// zoom of other edits (a slot moved whole, a cut of two clips, a level line).
    pub(crate) fn slot_zoom(&self, view: &SegmentsView, cuts: &[EditStill]) -> Option<Vec<EditStill>> {
        if self.slot_move.is_some() || self.level_line.is_some() {
            return None;
        }
        let stored = &self.stored_slots;
        let opened = self.trim_slot.as_ref().and_then(|id| stored.iter().find(|slot| &slot.slot_id == id));
        let (slot, taken) = if let Some(trim) = &self.slot_trim {
            (stored.iter().find(|slot| slot.slot_id == trim.slot_id)?, SlotZoomTaken::Strip(trim.start))
        } else if let Some((marker_id, _)) = &self.marker_edit {
            // The slot opened, when the marker is at one of its edges; else the slot it starts,
            // else the one it ends.
            let at_edge = |slot: &&qnc_program_db::ProgramSlot| slot.start_marker_id == *marker_id || slot.end_marker_id == *marker_id;
            let slot = opened.filter(at_edge).or_else(|| stored.iter().find(|slot| slot.start_marker_id == *marker_id)).or_else(|| stored.iter().find(|slot| slot.end_marker_id == *marker_id))?;
            let edge = slot.start_marker_id == *marker_id;
            let taken = match self.cut_part {
                cut_part::CutPart::Whole => SlotZoomTaken::Marker(edge),
                cut_part::CutPart::Picture => SlotZoomTaken::Picture(edge),
                cut_part::CutPart::Sound { lane, .. } => SlotZoomTaken::Sound(edge, lane),
            };
            (slot, taken)
        } else {
            (opened?, SlotZoomTaken::Nothing)
        };
        // The locked start and end of a story have no M marker to roll (an S-Story's do: its
        // clips are its slots).
        let usable = |marker_id: &str| {
            let locked = view.markers.iter().any(|marker| marker.marker_id == marker_id && marker.locked);
            (!locked || view.clips_are_slots).then(|| marker_id.to_string())
        };
        let markers = [usable(&slot.start_marker_id), usable(&slot.end_marker_id)];
        let markers = [markers[0].as_deref(), markers[1].as_deref()];
        let mut stills = qnc_program_view::slot_zoom_stills(view, &slots(stored, None), &slot.slot_id, markers, taken);
        // The window of the chosen edge shows the edge as drafted (the edit's own picture on its
        // side of the cut), so the end of a clip is seen there.
        if let Some(edge) = taken.edge() {
            let window = if edge { 0 } else { 2 };
            if let Some(cut) = cuts.iter().find(|still| still.after == edge) {
                let shown = &mut stills[window];
                (shown.picture, shown.id, shown.clip_edge, shown.clip_frames, shown.past_end) = (cut.picture.clone(), cut.id.clone(), cut.clip_edge, cut.clip_frames, cut.past_end);
            }
        }
        Some(stills)
    }
}
