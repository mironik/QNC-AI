//! The conductor of an ingest (user 2026-11-17: "dodaj evidenciju obrađenih klipova i na osnovu
//! toga dodaji nekome veći prioritet dinamički; cilj je da AI prijepis, val i filmstrip imaju
//! približno isti broj obrađenih klipova; ako nekome u nekom trenutku trebaš dati 90 % ukupne
//! snage procesora, daj mu; ovaj proces treba završiti maksimalno brzo s maksimalno potrebnim
//! opterećenjem procesora"). The steps of the background work work in blocks (a clip, a piece of
//! speech) and ask the conductor before each block how many they may work on at once (their
//! places). It is told, for each step, how many clips it has done and how many it has left.
//!
//! The rule, every period:
//! - The balanced steps (the transcription, the waves, the filmstrips) keep about the same count
//!   of clips done: a step more than `AHEAD` clips ahead of the one behind waits (no place), so
//!   the one behind gets the whole processor; every other step with work gets all its places, so
//!   the processor is full.
//! - A step not balanced (the AI) gets all its places whenever it has work (the worker holds it
//!   back until the balanced ones are done).
//!
//! It only decides; it reads nothing, starts nothing and keeps nothing between runs. The clock is
//! given (seconds), so the rule is tested without waiting.

/// How often the places are given anew.
pub const PERIOD_SECONDS: f64 = 10.0;
/// How many clips a balanced step may be ahead of the one behind before it waits.
pub const AHEAD: usize = 2;

/// One step: its code, how many blocks it can work on at once at most, and whether its count of
/// clips done is kept level with the other balanced steps.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub code: String,
    pub most: usize,
    pub balanced: bool,
}

impl Step {
    pub fn new(code: &str, most: usize) -> Self {
        Self { code: code.to_string(), most: most.max(1), balanced: false }
    }

    /// Its count of clips done is kept level with the other balanced steps.
    pub fn balanced(mut self) -> Self {
        self.balanced = true;
        self
    }
}

/// What a period decided, for the log: each step with its places, clips done and clips left.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub steps: Vec<(String, usize, usize, usize)>,
}

#[derive(Debug, Clone)]
struct State {
    step: Step,
    done: usize,
    left: usize,
    places: usize,
}

/// The conductor of one run.
#[derive(Debug, Clone)]
pub struct Conductor {
    steps: Vec<State>,
    given: f64,
}

impl Conductor {
    /// The steps (a code twice is an error).
    pub fn new(steps: Vec<Step>) -> Result<Self, String> {
        let mut seen = std::collections::BTreeSet::new();
        for step in &steps {
            if !seen.insert(step.code.clone()) {
                return Err(format!("conductor: step {} twice", step.code));
            }
        }
        Ok(Self { steps: steps.into_iter().map(|step| State { step, done: 0, left: 0, places: 0 }).collect(), given: f64::NEG_INFINITY })
    }

    fn at(&self, code: &str) -> Option<usize> {
        self.steps.iter().position(|state| state.step.code == code)
    }

    /// How many clips `code` has done in all and how many it has left now.
    pub fn work(&mut self, code: &str, done: usize, left: usize) {
        if let Some(i) = self.at(code) {
            (self.steps[i].done, self.steps[i].left) = (done, left);
        }
    }

    /// How many blocks `code` may work on at once now (a step the conductor does not know: none).
    pub fn places(&self, code: &str) -> usize {
        self.at(code).map_or(0, |i| self.steps[i].places)
    }

    /// One tick at `now` (seconds): a step with nothing left holds no place at once, and every
    /// period the places are given anew (what was given, for the log).
    pub fn tick(&mut self, now: f64) -> Option<Decision> {
        for state in &mut self.steps {
            if state.left == 0 {
                state.places = 0;
            }
        }
        if now - self.given < PERIOD_SECONDS {
            return None;
        }
        self.given = now;
        // The balanced step behind: the fewest clips done among those with clips left.
        let behind = self.steps.iter().filter(|state| state.step.balanced && state.left > 0).map(|state| state.done).min();
        for state in &mut self.steps {
            state.places = match (state.left, behind) {
                (0, _) => 0,
                (_, Some(behind)) if state.step.balanced && state.done > behind + AHEAD => 0,
                _ => state.step.most,
            };
        }
        Some(Decision { steps: self.steps.iter().map(|state| (state.step.code.clone(), state.places, state.done, state.left)).collect() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest() -> Conductor {
        Conductor::new(vec![Step::new("transcripts", 1).balanced(), Step::new("waves", 4).balanced(), Step::new("filmstrips", 8).balanced(), Step::new("shots", 1)]).unwrap()
    }

    #[test]
    fn a_step_ahead_waits_and_the_one_behind_gets_the_whole_processor() {
        let mut c = ingest();
        c.work("transcripts", 40, 60);
        c.work("waves", 90, 10);
        c.work("filmstrips", 20, 80);
        c.tick(0.0);
        assert_eq!((c.places("transcripts"), c.places("waves"), c.places("filmstrips")), (0, 0, 8), "the filmstrips behind: they alone work");
        c.work("filmstrips", 39, 61);
        c.tick(10.0);
        assert_eq!((c.places("transcripts"), c.places("waves"), c.places("filmstrips")), (1, 0, 8), "within two clips: both work");
    }

    #[test]
    fn a_step_with_nothing_left_is_not_the_one_behind() {
        let mut c = ingest();
        // The waves have every clip copied so far: the transcription and the filmstrips go on.
        c.work("transcripts", 30, 20);
        c.work("waves", 50, 0);
        c.work("filmstrips", 31, 19);
        c.tick(0.0);
        assert_eq!((c.places("transcripts"), c.places("waves"), c.places("filmstrips")), (1, 0, 8));
    }

    #[test]
    fn a_step_not_balanced_gets_all_its_places_with_work_and_none_without() {
        let mut c = ingest();
        c.work("shots", 0, 5);
        c.tick(0.0);
        assert_eq!(c.places("shots"), 1);
        c.work("shots", 5, 0);
        c.tick(1.0);
        assert_eq!(c.places("shots"), 0, "at once, not at the next period");
    }

    #[test]
    fn a_step_twice_is_refused_and_an_unknown_step_has_no_place() {
        assert!(Conductor::new(vec![Step::new("waves", 1), Step::new("waves", 2)]).is_err());
        assert_eq!(ingest().places("teleport"), 0);
    }
}
