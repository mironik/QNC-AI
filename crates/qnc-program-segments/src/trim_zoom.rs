//! The trim zoom of a clip (AGENTS 3a "Model reza i zoom", user 2026-11-26): a clip is IN - SLOT -
//! OUT, its slot its length. The IN picture, the slot (the player, one length strip under it), the
//! OUT picture; the M markers (OUT/IN with the neighbours) on the outer edges. IN or OUT changes this
//! clip's length alone (the neighbours keep theirs and move), an M both clips' lengths (roll). The
//! sound lanes under IN and OUT are those M markers' sound (J/L). Which clip and what is chosen
//! follow from what is taken.

use super::*;
use qnc_program_view::{ClipZoomEdge, ClipZoomTaken, EditStill, StillAction, StillStrip};

impl ProgramSegments {
    /// An element of the zoom of a clip clicked: one click, one action (AGENTS 3a). `kind` edge
    /// (the clip's own IN or OUT, `id` its slot), marker (the M with that neighbour, `id` the
    /// marker), a1 or a2 (that M marker's sound).
    pub(crate) fn zoom_cut_element(&mut self, kind: &str, right: bool, id: &str) -> bool {
        self.let_go_if_unmoved();
        match kind {
            "edge" => {
                self.selected_slot = Some(id.to_string());
                self.take_slot_edge(!right);
            }
            "marker" => {
                self.select_marker(id);
                self.edit_marker();
            }
            "a1" | "a2" => {
                self.take_cut_part(Some(id.to_string()), false, u8::from(kind == "a2"));
            }
            _ => return false,
        }
        true
    }

    /// The zoom of the clip, when one is open or something at its edges is taken; `None`: the zoom
    /// of other edits (a slot moved whole, a level line, a cut of two clips under no M marker).
    pub(crate) fn slot_zoom(&self, view: &SegmentsView, cuts: &[EditStill]) -> Option<Vec<EditStill>> {
        if self.slot_move.is_some() || self.level_line.is_some() {
            return None;
        }
        let stored = &self.stored_slots;
        let opened = self.zoom_clip.as_ref().and_then(|id| stored.iter().find(|slot| &slot.slot_id == id));
        let (slot, taken) = if let Some(trim) = &self.slot_trim {
            (stored.iter().find(|slot| slot.slot_id == trim.slot_id)?, ClipZoomTaken::Edge { right: !trim.start })
        } else if let Some((marker_id, _)) = &self.marker_edit {
            // The clip shown, when the marker is one of its M markers; else the clip it starts,
            // else the one it ends.
            let at_edge = |slot: &&qnc_program_db::ProgramSlot| slot.start_marker_id == *marker_id || slot.end_marker_id == *marker_id;
            let slot = opened.filter(at_edge).or_else(|| stored.iter().find(|slot| slot.start_marker_id == *marker_id)).or_else(|| stored.iter().find(|slot| slot.end_marker_id == *marker_id))?;
            let right = slot.end_marker_id == *marker_id;
            let taken = match self.cut_part {
                cut_part::CutPart::Sound { lane, .. } => ClipZoomTaken::Sound { right, lane },
                _ => ClipZoomTaken::Marker { right },
            };
            (slot, taken)
        } else {
            (opened?, ClipZoomTaken::Nothing)
        };
        // The locked start and end of a story have no M marker to roll (an S-Story's do).
        let usable = |marker_id: &str| {
            let locked = view.markers.iter().any(|marker| marker.marker_id == marker_id && marker.locked);
            (!locked || view.clips_are_slots).then(|| marker_id.to_string())
        };
        let action = |kind: &str, right: bool, id: &str| StillAction::new("zoom_cut_element", Some(format!("{kind}:{}:{id}", u8::from(right))), None);
        let last = slot.end_frame.saturating_sub(1).max(slot.start_frame);
        // An edge's picture as drafted when the edit moved it (the clip's own side of the cut).
        let drafted = |right: bool| cuts.iter().find(|still| still.after != right).and_then(|still| still.picture.clone());
        let edge = |right: bool| {
            let marker = usable(if right { &slot.end_marker_id } else { &slot.start_marker_id });
            let stored_picture = qnc_program_view::program_picture(view, if right { last } else { slot.start_frame });
            let moved = taken.window() == if right { 2 } else { 0 } && taken != ClipZoomTaken::Nothing;
            ClipZoomEdge {
                picture: if moved { drafted(right).or(stored_picture) } else { stored_picture },
                frame: if right { slot.end_frame } else { slot.start_frame },
                take: action("edge", right, &slot.slot_id),
                sounds: marker.as_deref().map_or([None, None], |marker| [Some(action("a1", right, marker)), Some(action("a2", right, marker))]),
                marker: marker.map(|marker| action("marker", right, &marker)),
            }
        };
        let frames = slot.end_frame.saturating_sub(slot.start_frame);
        let label = view.timebase.map_or_else(String::new, |(num, den)| qnc_program_view::duration_label(frames, num, den));
        let strip = StillStrip { label, frames, fill: None, chosen: false, select: None, take: action("edge", false, &slot.slot_id) };
        let playhead = view.playhead.unwrap_or(slot.start_frame).clamp(slot.start_frame, last);
        Some(qnc_program_view::clip_zoom_stills(edge(false), edge(true), strip, (playhead, qnc_program_view::program_picture(view, playhead)), taken))
    }
}
