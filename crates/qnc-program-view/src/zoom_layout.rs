//! The three windows of a zoom (AGENTS 3a "Model reza i zoom", user 2026-11-26): what each
//! window shows follows from what is taken. The middle is always the player on the edge that
//! moves; the outer window on the side of the cut is the neighbour's edge there, the other is the
//! opposite edge of the chosen slot. Where there is no neighbour (the first or last slot, a source
//! clip) the window shows the clip beyond the cut, greyed. What is not active is greyed.
//!
//! The owner of an axis (source timeline, Wrap rows, composite statement) fills the pictures; this
//! cube only says which place each window is, so the zoom is the same everywhere.

/// What is taken at a cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomTaken {
    /// Nothing: the zoom opened on the slot.
    Nothing,
    /// The IN or the OUT edge of the slot (ripple).
    SlotIn,
    SlotOut,
    /// J/L of the picture: the J of the clip at its IN, the L of the clip at its OUT.
    PictureJ,
    PictureL,
    /// J/L of the sound: as the picture's, the pictures stand and the wave is the active thing.
    SoundJ,
    SoundL,
    /// The M marker line (roll): the slot before and the slot after.
    Roll,
}

/// The place a window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomPlace {
    /// The player at the playhead (nothing taken).
    Playhead,
    /// The edges of the chosen slot.
    SlotIn,
    SlotOut,
    /// The OUT of the slot before, the IN of the slot after.
    PreviousOut,
    NextIn,
    /// The picture before the IN, after the OUT, where there is no neighbour.
    BeyondIn,
    BeyondOut,
    /// The L of the clip (its picture past its OUT) and the J of the next clip.
    ClipL,
    NextJ,
    /// The L of the clip before and the J of the clip (its picture before its IN).
    PreviousL,
    ClipJ,
    /// The IN of the slot before and the OUT of the slot after (a roll).
    PreviousSlotIn,
    NextSlotOut,
    /// The cut itself (a roll).
    Cut,
}

/// One window of the zoom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomWindow {
    pub place: ZoomPlace,
    /// The window is the player (the monitor's picture as it plays and moves).
    pub player: bool,
    /// What moves is in this window: framed red; every other window is greyed.
    pub active: bool,
}

/// The three windows and, for J/L of the sound, the side of the wave that is active
/// (`Some(false)` before the cut, `Some(true)` after it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoomLayout {
    pub windows: [ZoomWindow; 3],
    pub wave_active: Option<bool>,
}

impl ZoomLayout {
    /// The windows for what is taken; `previous` and `next`: the slot has a neighbour there.
    pub fn of(taken: ZoomTaken, previous: bool, next: bool) -> Self {
        let window = |place, player, active| ZoomWindow { place, player, active };
        let outer = |place| window(place, false, false);
        let middle = |place| window(place, true, true);
        let before = if previous { ZoomPlace::PreviousOut } else { ZoomPlace::BeyondIn };
        let after = if next { ZoomPlace::NextIn } else { ZoomPlace::BeyondOut };
        let (windows, wave_active) = match taken {
            ZoomTaken::Nothing => ([outer(ZoomPlace::SlotIn), middle(ZoomPlace::Playhead), outer(ZoomPlace::SlotOut)], None),
            ZoomTaken::SlotIn => ([outer(before), middle(ZoomPlace::SlotIn), outer(ZoomPlace::SlotOut)], None),
            ZoomTaken::SlotOut => ([outer(ZoomPlace::SlotIn), middle(ZoomPlace::SlotOut), outer(after)], None),
            ZoomTaken::PictureJ => ([outer(if previous { ZoomPlace::PreviousL } else { ZoomPlace::BeyondIn }), middle(ZoomPlace::ClipJ), outer(ZoomPlace::SlotOut)], None),
            ZoomTaken::PictureL => ([outer(ZoomPlace::SlotIn), middle(ZoomPlace::ClipL), outer(if next { ZoomPlace::NextJ } else { ZoomPlace::BeyondOut })], None),
            // The pictures stand (the edge of the slot, not moving): the wave is what is active.
            ZoomTaken::SoundJ => ([outer(before), window(ZoomPlace::SlotIn, true, false), outer(ZoomPlace::SlotOut)], Some(true)),
            ZoomTaken::SoundL => ([outer(ZoomPlace::SlotIn), window(ZoomPlace::SlotOut, true, false), outer(after)], Some(false)),
            ZoomTaken::Roll => ([outer(ZoomPlace::PreviousSlotIn), middle(ZoomPlace::Cut), outer(ZoomPlace::NextSlotOut)], None),
        };
        Self { windows, wave_active }
    }

    /// The place of each window, left to right.
    pub fn places(&self) -> [ZoomPlace; 3] {
        self.windows.map(|window| window.place)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ZoomPlace as P;

    /// The table of AGENTS 3a: every taken state, with and without neighbours.
    #[test]
    fn the_windows_follow_what_is_taken_as_the_rule_says() {
        let table = [
            (ZoomTaken::Nothing, true, true, [P::SlotIn, P::Playhead, P::SlotOut]),
            (ZoomTaken::SlotOut, true, true, [P::SlotIn, P::SlotOut, P::NextIn]),
            (ZoomTaken::SlotIn, true, true, [P::PreviousOut, P::SlotIn, P::SlotOut]),
            (ZoomTaken::PictureL, true, true, [P::SlotIn, P::ClipL, P::NextJ]),
            (ZoomTaken::PictureJ, true, true, [P::PreviousL, P::ClipJ, P::SlotOut]),
            (ZoomTaken::SoundL, true, true, [P::SlotIn, P::SlotOut, P::NextIn]),
            (ZoomTaken::SoundJ, true, true, [P::PreviousOut, P::SlotIn, P::SlotOut]),
            (ZoomTaken::Roll, true, true, [P::PreviousSlotIn, P::Cut, P::NextSlotOut]),
            // No neighbour: the picture beyond the cut.
            (ZoomTaken::SlotIn, false, false, [P::BeyondIn, P::SlotIn, P::SlotOut]),
            (ZoomTaken::SlotOut, false, false, [P::SlotIn, P::SlotOut, P::BeyondOut]),
            (ZoomTaken::PictureJ, false, true, [P::BeyondIn, P::ClipJ, P::SlotOut]),
            (ZoomTaken::PictureL, true, false, [P::SlotIn, P::ClipL, P::BeyondOut]),
        ];
        for (taken, previous, next, places) in table {
            assert_eq!(ZoomLayout::of(taken, previous, next).places(), places, "{taken:?} previous={previous} next={next}");
        }
    }

    #[test]
    fn the_middle_is_the_player_and_only_what_moves_is_active() {
        for taken in [ZoomTaken::Nothing, ZoomTaken::SlotIn, ZoomTaken::SlotOut, ZoomTaken::PictureJ, ZoomTaken::PictureL, ZoomTaken::Roll] {
            let layout = ZoomLayout::of(taken, true, true);
            assert!(layout.windows[1].player && layout.windows[1].active, "{taken:?}: the middle plays and is active");
            assert!(!layout.windows[0].active && !layout.windows[2].active, "{taken:?}: the outer windows are greyed");
            assert_eq!(layout.wave_active, None);
        }
        for (taken, side) in [(ZoomTaken::SoundJ, true), (ZoomTaken::SoundL, false)] {
            let layout = ZoomLayout::of(taken, true, true);
            assert!(layout.windows.iter().all(|window| !window.active), "{taken:?}: the pictures stand, greyed");
            assert_eq!(layout.wave_active, Some(side), "{taken:?}: the wave is active");
        }
    }
}
