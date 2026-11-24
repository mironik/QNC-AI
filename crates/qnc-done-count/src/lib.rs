//! What the steps of this process finished, clip by clip (user 2026-11-17: "ne brojati, treba
//! aplikacija samo poslati u array za svaki odrađeni +1... to je vjerojatno najjeftinije"). A step
//! adds one for every clip it finishes; who leads the steps reads the counts instead of counting
//! the base's database again and again. Only this process: nothing is kept, nothing is written.

use std::collections::BTreeMap;
use std::sync::Mutex;

/// The clips copied into the project.
pub const IMPORTED: &str = "imported";
/// The clips whose first channel is heard (transcribed, or found to have nothing to hear).
pub const TRANSCRIPTS: &str = "transcripts";
/// The clips with their wave of every frame.
pub const WAVES: &str = "waves";
/// The clips with their filmstrip.
pub const FILMSTRIPS: &str = "filmstrips";

static COUNTS: Mutex<BTreeMap<&'static str, usize>> = Mutex::new(BTreeMap::new());

/// `step` finished one more clip.
pub fn add(step: &'static str) {
    *COUNTS.lock().unwrap_or_else(|e| e.into_inner()).entry(step).or_default() += 1;
}

/// How many clips `step` finished in this process so far.
pub fn count(step: &str) -> usize {
    COUNTS.lock().unwrap_or_else(|e| e.into_inner()).get(step).copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_clip_finished_adds_one() {
        let before = super::count("test-step");
        super::add("test-step");
        super::add("test-step");
        assert_eq!(super::count("test-step"), before + 2);
        assert_eq!(super::count("never"), 0);
    }
}
