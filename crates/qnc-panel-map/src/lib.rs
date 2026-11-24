//! Where the panels of a form are on screen (user 2026-11-15: "Alt + strelice za izbor panela,
//! strelice ostaju za šetnju unutar panela; nema kruga, korisnik određuje gdje želi"). The
//! board that draws the form writes the places of its panels, the keys read them: Alt + an
//! arrow goes to the nearest panel that way, a click anywhere in a panel focuses it. The same
//! for every form; which places are panels is the form's.

/// A rectangle on screen: left, top, right, bottom.
pub type Place = [f32; 4];

/// Where an arrow points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    /// The direction of a catalog action (Alt + an arrow).
    pub fn of_action(action_id: &str) -> Option<Self> {
        Some(match action_id {
            PANEL_LEFT => Self::Left,
            PANEL_RIGHT => Self::Right,
            PANEL_UP => Self::Up,
            PANEL_DOWN => Self::Down,
            _ => return None,
        })
    }
}

/// The catalog actions of Alt + the arrows, and of a click in a panel (its target the panel).
pub const PANEL_LEFT: &str = "panel_left";
pub const PANEL_RIGHT: &str = "panel_right";
pub const PANEL_UP: &str = "panel_up";
pub const PANEL_DOWN: &str = "panel_down";
pub const FOCUS_PANEL: &str = "focus_panel";

/// The places of the panels as the board last drew them, by name; a handle shared by the
/// clones of a view, not part of what the view says (two are equal whatever they hold).
#[derive(Debug, Clone, Default)]
pub struct PanelPlaces(std::sync::Arc<std::sync::Mutex<Vec<(String, Place)>>>);

impl PanelPlaces {
    pub fn set(&self, places: Vec<(String, Place)>) {
        if let Ok(mut shown) = self.0.lock() {
            *shown = places;
        }
    }

    pub fn get(&self) -> Vec<(String, Place)> {
        self.0.lock().map(|places| places.clone()).unwrap_or_default()
    }
}

impl PartialEq for PanelPlaces {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// The panel in `direction` from the panel `from`: of those that lie wholly beyond its edge
/// that way (live 2026-11-15: the source timeline under the whole width was "left" of the
/// transcript), the nearest, the one that overlaps it across the direction first; `None` when
/// there is none.
pub fn beside(places: &[(String, Place)], from: &str, direction: Direction) -> Option<String> {
    let (_, at) = places.iter().find(|(name, _)| name == from)?;
    // A border drawn between two panels may overlap them by a pixel or two.
    const SLACK: f32 = 4.0;
    places
        .iter()
        .filter(|(name, _)| name != from)
        .filter_map(|(name, place)| {
            // The gap along the direction and how far apart the two are across it.
            let (along, across) = match direction {
                Direction::Left if place[2] <= at[0] + SLACK => (at[0] - place[2], gap(at[1], at[3], place[1], place[3])),
                Direction::Right if place[0] >= at[2] - SLACK => (place[0] - at[2], gap(at[1], at[3], place[1], place[3])),
                Direction::Up if place[3] <= at[1] + SLACK => (at[1] - place[3], gap(at[0], at[2], place[0], place[2])),
                Direction::Down if place[1] >= at[3] - SLACK => (place[1] - at[3], gap(at[0], at[2], place[0], place[2])),
                _ => return None,
            };
            // Of the same gaps, the one that overlaps it most across.
            let overlap = match direction {
                Direction::Left | Direction::Right => at[3].min(place[3]) - at[1].max(place[1]),
                Direction::Up | Direction::Down => at[2].min(place[2]) - at[0].max(place[0]),
            };
            Some((across, along.max(0.0), -overlap, name))
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, _, _, name)| name.clone())
}

/// How far apart two ranges are (0 where they overlap).
fn gap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (b0 - a1).max(a0 - b1).max(0.0)
}

/// The panel under the point (`x`, `y`).
pub fn under(places: &[(String, Place)], x: f32, y: f32) -> Option<String> {
    places.iter().find(|(_, place)| x >= place[0] && x <= place[2] && y >= place[1] && y <= place[3]).map(|(name, _)| name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_arrow_goes_to_the_nearest_panel_that_way_and_a_point_to_the_panel_under_it() {
        // Pool tabs over the pool on the left, the transcript on the right, the source timeline
        // under both.
        let places: Vec<(String, Place)> = vec![
            ("pool_tabs".into(), [0.0, 0.0, 400.0, 30.0]),
            ("pool".into(), [0.0, 30.0, 400.0, 500.0]),
            ("transcript".into(), [400.0, 0.0, 900.0, 500.0]),
            ("source_timeline".into(), [0.0, 500.0, 900.0, 700.0]),
        ];
        assert_eq!(beside(&places, "pool", Direction::Right).as_deref(), Some("transcript"));
        assert_eq!(beside(&places, "transcript", Direction::Left).as_deref(), Some("pool"), "the one beside it across, not the tabs above");
        let wide: Vec<(String, Place)> = vec![("pool".into(), [0.0, 0.0, 400.0, 500.0]), ("transcript".into(), [400.0, 0.0, 900.0, 700.0]), ("source_timeline".into(), [0.0, 500.0, 900.0, 700.0])];
        assert_eq!(beside(&wide, "transcript", Direction::Left).as_deref(), Some("pool"), "not the panel under both");
        assert_eq!(beside(&places, "pool", Direction::Up).as_deref(), Some("pool_tabs"));
        assert_eq!(beside(&places, "pool", Direction::Down).as_deref(), Some("source_timeline"));
        assert_eq!(beside(&places, "source_timeline", Direction::Down), None, "nothing below: no round walk");
        assert_eq!(under(&places, 600.0, 100.0).as_deref(), Some("transcript"));
        assert_eq!(Direction::of_action(PANEL_UP), Some(Direction::Up));
    }
}
