//! The conductor of an ingest (user 2026-11-17: "cilj je da AI prijepis, val i filmstrip imaju
//! približno isti broj obrađenih klipova; ako nekome trebaš dati 90 % snage procesora, daj mu;
//! ovaj proces treba završiti maksimalno brzo s maksimalnim opterećenjem procesora"; "zaboravi
//! sva pravila, daj najbrže rješenje, nađi način da bude u isto vrijeme gotovo").
//!
//! The steps work in blocks (a clip, a piece of speech) and ask before each block how many they
//! may work on at once (their places). Every period the conductor is told each step's clips done
//! and left, measures how many clips a place does a second, and gives the places anew:
//! - every step with work holds one place (none stops: a stop and a start cost more than they
//!   give);
//! - the rest of the processor (its threads; a place costs the threads it uses) goes one place at
//!   a time to the step that would have done the fewest clips by the next look, so the counts of
//!   clips done stay level and the processor stays full.
//!
//! It only decides; it reads nothing, starts nothing and keeps nothing between runs. The clock is
//! given (seconds), so the rule is tested without waiting.

use std::collections::VecDeque;

/// How often the places are given anew.
pub const PERIOD_SECONDS: f64 = 2.0;
/// How far ahead the clips done are foreseen when the places are given.
pub const HORIZON_SECONDS: f64 = 10.0;
/// Over how long a place's speed is measured.
pub const WINDOW_SECONDS: f64 = 30.0;
/// Clips a place does a second until its own speed is measured.
pub const FIRST_SPEED: f64 = 0.2;

/// One step: its code, how many blocks it can work on at once at most, what a place costs of the
/// processor (threads), and whether its clips done are kept level with the others.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub code: String,
    pub most: usize,
    pub cost: usize,
    pub balanced: bool,
}

impl Step {
    pub fn new(code: &str, most: usize) -> Self {
        Self { code: code.to_string(), most: most.max(1), cost: 1, balanced: false }
    }

    /// A place costs `cost` threads of the processor.
    pub fn costing(mut self, cost: usize) -> Self {
        self.cost = cost.max(1);
        self
    }

    /// Its count of clips done is kept level with the other balanced steps.
    pub fn balanced(mut self) -> Self {
        self.balanced = true;
        self
    }
}

/// What a period decided, for the log: each step with its places, clips done, clips left and the
/// clips a place does a second (when measured).
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub steps: Vec<(String, usize, usize, usize, Option<f64>)>,
}

#[derive(Debug, Clone)]
struct State {
    step: Step,
    done: usize,
    left: usize,
    places: usize,
    /// Place-seconds held in all.
    held: f64,
    /// (seconds, clips done, place-seconds held) over the window.
    samples: VecDeque<(f64, usize, f64)>,
}

impl State {
    /// Clips one place does a second, measured.
    fn speed(&self) -> Option<f64> {
        let (first, last) = (self.samples.front()?, self.samples.back()?);
        let held = last.2 - first.2;
        let done = last.1.saturating_sub(first.1);
        (held >= 2.0 && done > 0).then(|| done as f64 / held)
    }

    /// The clips it would have done by the horizon with `places`.
    fn foreseen(&self, places: usize) -> f64 {
        let ahead = self.speed().unwrap_or(FIRST_SPEED) * places as f64 * HORIZON_SECONDS;
        self.done as f64 + ahead.min(self.left as f64)
    }
}

/// The conductor of one run.
#[derive(Debug, Clone)]
pub struct Conductor {
    steps: Vec<State>,
    threads: usize,
    given: f64,
    last: f64,
}

impl Conductor {
    /// The steps (a code twice is an error) and the processor's threads.
    pub fn new(steps: Vec<Step>, threads: usize) -> Result<Self, String> {
        let mut seen = std::collections::BTreeSet::new();
        for step in &steps {
            if !seen.insert(step.code.clone()) {
                return Err(format!("conductor: step {} twice", step.code));
            }
        }
        Ok(Self {
            steps: steps.into_iter().map(|step| State { step, done: 0, left: 0, places: 0, held: 0.0, samples: VecDeque::new() }).collect(),
            threads: threads.max(1),
            given: f64::NEG_INFINITY,
            last: 0.0,
        })
    }

    fn at(&self, code: &str) -> Option<usize> {
        self.steps.iter().position(|state| state.step.code == code)
    }

    /// How many clips `code` has done in all and how many it has left now.
    pub fn work(&mut self, code: &str, done: usize, left: usize) {
        if let Some(i) = self.at(code) {
            let now = self.last;
            let state = &mut self.steps[i];
            (state.done, state.left) = (done, left);
            let held = state.held;
            state.samples.push_back((now, done, held));
            while state.samples.front().is_some_and(|sample| now - sample.0 > WINDOW_SECONDS) {
                state.samples.pop_front();
            }
        }
    }

    /// How many blocks `code` may work on at once now (a step the conductor does not know: none).
    pub fn places(&self, code: &str) -> usize {
        self.at(code).map_or(0, |i| self.steps[i].places)
    }

    /// One tick at `now` (seconds): every period the places are given anew (what was given, for
    /// the log).
    pub fn tick(&mut self, now: f64) -> Option<Decision> {
        let dt = (now - self.last).max(0.0);
        self.last = now;
        for state in &mut self.steps {
            state.held += state.places as f64 * dt;
        }
        if now - self.given < PERIOD_SECONDS {
            return None;
        }
        self.given = now;
        let mut places: Vec<usize> = self.steps.iter().map(|state| usize::from(state.left > 0)).collect();
        // A step not balanced (the AI, held back by the worker until the others are done) takes
        // all its places.
        for (i, state) in self.steps.iter().enumerate() {
            if !state.step.balanced && state.left > 0 {
                places[i] = state.step.most;
            }
        }
        let mut spent: usize = places.iter().zip(&self.steps).map(|(p, state)| p * state.step.cost).sum();
        loop {
            let next = (0..self.steps.len())
                .filter(|&i| {
                    let state = &self.steps[i];
                    state.step.balanced && state.left > places[i] && places[i] < state.step.most && spent + state.step.cost <= self.threads
                })
                .min_by(|&a, &b| self.steps[a].foreseen(places[a]).total_cmp(&self.steps[b].foreseen(places[b])));
            let Some(i) = next else { break };
            places[i] += 1;
            spent += self.steps[i].step.cost;
        }
        for (state, places) in self.steps.iter_mut().zip(places) {
            state.places = places;
        }
        Some(Decision { steps: self.steps.iter().map(|state| (state.step.code.clone(), state.places, state.done, state.left, state.speed())).collect() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest() -> Conductor {
        Conductor::new(
            vec![
                Step::new("transcripts", 3).costing(2).balanced(),
                Step::new("waves", 4).balanced(),
                Step::new("filmstrips", 8).balanced(),
                Step::new("shots", 1),
            ],
            8,
        )
        .unwrap()
    }

    /// `seconds` ticks a second apart; each step does `speed` clips a second per place.
    fn run(c: &mut Conductor, from: f64, seconds: usize, left: &mut [usize; 3], done: &mut [usize; 3], speed: [f64; 3]) {
        let codes = ["transcripts", "waves", "filmstrips"];
        let mut carry = [0.0; 3];
        for s in 0..seconds {
            for i in 0..3 {
                carry[i] += speed[i] * c.places(codes[i]) as f64;
                let did = (carry[i].floor() as usize).min(left[i]);
                carry[i] -= did as f64;
                left[i] -= did;
                done[i] += did;
                c.work(codes[i], done[i], left[i]);
            }
            c.tick(from + s as f64);
        }
    }

    #[test]
    fn the_counts_of_clips_done_draw_level_and_the_processor_stays_full() {
        let mut c = ingest();
        let (mut left, mut done) = ([103, 103, 103], [0, 0, 0]);
        // A place of the transcription slow, of the waves fast, of the filmstrips between.
        run(&mut c, 0.0, 60, &mut left, &mut done, [0.08, 0.6, 0.15]);
        // The costly ones level; the waves, cheap on one place, may run ahead (no stop for them).
        assert!(done[0].abs_diff(done[2]) <= 4 && done[1] >= done[0], "done {done:?}");
        let threads = c.places("transcripts") * 2 + c.places("waves") + c.places("filmstrips");
        assert_eq!(threads, 8, "the processor full");
        assert!(c.places("waves") == 1, "the fast waves need no more than one");
    }

    #[test]
    fn every_step_with_work_holds_a_place_and_none_without() {
        let mut c = ingest();
        c.work("transcripts", 50, 10);
        c.work("waves", 103, 0);
        c.work("filmstrips", 10, 93);
        c.tick(0.0);
        assert_eq!((c.places("transcripts"), c.places("waves")), (1, 0));
        assert!(c.places("filmstrips") >= 5, "the one behind takes the rest");
    }

    #[test]
    fn a_step_not_balanced_takes_all_its_places_with_work() {
        let mut c = ingest();
        c.work("shots", 0, 5);
        c.tick(0.0);
        assert_eq!(c.places("shots"), 1);
    }

    #[test]
    fn a_step_twice_is_refused_and_an_unknown_step_has_no_place() {
        assert!(Conductor::new(vec![Step::new("waves", 1), Step::new("waves", 2)], 8).is_err());
        assert_eq!(ingest().places("teleport"), 0);
    }
}
