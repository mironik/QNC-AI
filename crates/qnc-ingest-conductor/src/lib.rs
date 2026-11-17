//! The conductor of an ingest (user 2026-11-17: "trebaš tražiti drugačiju organizaciju procesa i
//! optimizirati dinamičkim prilagođavanjem"; "orkestrator bi trebao biti dinamički i vidjeti što
//! kasni pa prilagoditi"). The steps of the background work (the transcription, the waves, the
//! filmstrips, the AI) work in blocks (a clip, a piece of speech) and ask the conductor before each
//! block how many they may work on at once (their places). It is told, for each step, how many
//! blocks are left and how many it has done, and how busy the processor is.
//!
//! The rule, every period:
//! - Each step with work holds the places it keeps (none waits whole: "da se ništa ne čeka
//!   previše").
//! - The processor is a budget: a place costs what the step uses of it (the transcription its
//!   threads). The budget grows while the processor has room and shrinks while it is full.
//! - What is left of the budget goes, one place at a time, to the step that would end last: its
//!   blocks left over what it does a second with the places it holds, times its weight (the
//!   transcription first: the journalist reads the statements first).
//!
//! It only decides; it reads nothing, starts nothing and keeps nothing between runs. The clock is
//! given (seconds), so the rule is tested without waiting.

use std::collections::VecDeque;

/// How often the places are given anew.
pub const PERIOD_SECONDS: f64 = 10.0;
/// Over how long a step's speed is measured.
pub const WINDOW_SECONDS: f64 = 30.0;
/// Above this share of the processor busy the budget shrinks by one, below `ROOM` it grows.
pub const FULL: f64 = 0.95;
pub const ROOM: f64 = 0.80;

/// One step: its code, how many blocks it can work on at once at most, how many places it keeps
/// whenever it has work, what a place costs of the processor and its weight.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub code: String,
    pub most: usize,
    pub least: usize,
    pub cost: usize,
    pub weight: f64,
}

impl Step {
    pub fn new(code: &str, most: usize) -> Self {
        Self { code: code.to_string(), most: most.max(1), least: 0, cost: 1, weight: 1.0 }
    }

    /// With work it holds `least` places at least.
    pub fn keeping(mut self, least: usize) -> Self {
        self.least = least.min(self.most);
        self
    }

    /// A place costs `cost` of the processor (its threads).
    pub fn costing(mut self, cost: usize) -> Self {
        self.cost = cost.max(1);
        self
    }

    /// Its time left counts `weight` times (above 1: it is given places first).
    pub fn weighing(mut self, weight: f64) -> Self {
        self.weight = weight.max(0.1);
        self
    }
}

/// What a period decided, for the log.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub budget: usize,
    pub busy: Option<f64>,
    /// Each step with work: its code, places, blocks left and the seconds it would still take.
    pub steps: Vec<(String, usize, usize, Option<f64>)>,
}

#[derive(Debug, Clone)]
struct State {
    step: Step,
    left: usize,
    /// (seconds, blocks done, place-seconds held) over the window.
    samples: VecDeque<(f64, usize, f64)>,
    held: f64,
    places: usize,
}

impl State {
    /// Blocks it does a second with one place (none yet: unknown).
    fn speed(&self) -> Option<f64> {
        let (first, last) = (self.samples.front()?, self.samples.back()?);
        let held = last.2 - first.2;
        let done = last.1.saturating_sub(first.1);
        (held > 1.0 && done > 0).then(|| done as f64 / held)
    }

    /// The seconds it would still take with `places` (weighted), at `speed` when its own is unknown.
    fn ends_in(&self, places: usize, default: f64) -> f64 {
        if self.left == 0 {
            return 0.0;
        }
        let speed = self.speed().unwrap_or(default);
        self.left as f64 / (speed * places.max(1) as f64) * self.step.weight
    }
}

/// The conductor of one run.
#[derive(Debug, Clone)]
pub struct Conductor {
    steps: Vec<State>,
    cores: usize,
    budget: usize,
    busy: Option<f64>,
    given: f64,
    last: f64,
}

impl Conductor {
    /// The steps in their order (a code twice is an error) and how many cores the processor has.
    pub fn new(steps: Vec<Step>, cores: usize) -> Result<Self, String> {
        let mut seen = std::collections::BTreeSet::new();
        for step in &steps {
            if !seen.insert(step.code.clone()) {
                return Err(format!("conductor: step {} twice", step.code));
            }
        }
        let cores = cores.max(1);
        Ok(Self {
            steps: steps.into_iter().map(|step| State { step, left: 0, samples: VecDeque::new(), held: 0.0, places: 0 }).collect(),
            cores,
            budget: cores,
            busy: None,
            given: f64::NEG_INFINITY,
            last: 0.0,
        })
    }

    fn at(&self, code: &str) -> Option<usize> {
        self.steps.iter().position(|state| state.step.code == code)
    }

    /// How many blocks `code` has left to do now, and how many it has done in all (it only grows).
    pub fn work(&mut self, code: &str, left: usize, done: usize) {
        if let Some(i) = self.at(code) {
            self.steps[i].left = left;
            let held = self.steps[i].held;
            let now = self.last;
            let samples = &mut self.steps[i].samples;
            samples.push_back((now, done, held));
            while samples.front().is_some_and(|sample| now - sample.0 > WINDOW_SECONDS) {
                samples.pop_front();
            }
        }
    }

    /// The share of the processor busy (0..1).
    pub fn busy(&mut self, share: f64) {
        self.busy = Some(share.clamp(0.0, 1.0));
    }

    /// How many blocks `code` may work on at once now (a step the conductor does not know: none).
    pub fn places(&self, code: &str) -> usize {
        self.at(code).map_or(0, |i| self.steps[i].places)
    }

    /// One tick at `now` (seconds): the places held count, and every period they are given anew
    /// (what was given, for the log).
    pub fn tick(&mut self, now: f64) -> Option<Decision> {
        let dt = (now - self.last).max(0.0);
        self.last = now;
        for state in &mut self.steps {
            state.held += state.places as f64 * dt;
            // No work, no place, at once: the next one does not wait for the period.
            if state.left == 0 {
                state.places = 0;
            }
        }
        if now - self.given < PERIOD_SECONDS {
            return None;
        }
        self.given = now;
        // The budget follows the processor.
        match self.busy {
            Some(busy) if busy >= FULL => self.budget = self.budget.saturating_sub(1),
            Some(busy) if busy < ROOM => self.budget = (self.budget + 1).min(self.cores),
            _ => {}
        }
        let floor: usize = self.steps.iter().filter(|state| state.left > 0).map(|state| state.step.least * state.step.cost).sum();
        self.budget = self.budget.max(floor);
        // A step whose speed is not known yet is taken at the slowest known (it gets places).
        let default = self.steps.iter().filter_map(State::speed).fold(f64::INFINITY, f64::min);
        let default = if default.is_finite() { default } else { 1.0 };
        let mut places: Vec<usize> = self.steps.iter().map(|state| if state.left > 0 { state.step.least } else { 0 }).collect();
        let mut spent: usize = places.iter().zip(&self.steps).map(|(p, state)| p * state.step.cost).sum();
        loop {
            let next = (0..self.steps.len())
                .filter(|&i| {
                    let state = &self.steps[i];
                    state.left > places[i] && places[i] < state.step.most && spent + state.step.cost <= self.budget
                })
                .max_by(|&a, &b| self.steps[a].ends_in(places[a], default).total_cmp(&self.steps[b].ends_in(places[b], default)));
            let Some(i) = next else { break };
            places[i] += 1;
            spent += self.steps[i].step.cost;
        }
        let mut steps = Vec::new();
        for (state, places) in self.steps.iter_mut().zip(places) {
            state.places = places;
            if state.left > 0 {
                let ends = state.speed().map(|speed| state.left as f64 / (speed * places.max(1) as f64));
                steps.push((state.step.code.clone(), places, state.left, ends));
            }
        }
        Some(Decision { budget: self.budget, busy: self.busy, steps })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest() -> Conductor {
        Conductor::new(
            vec![
                Step::new("transcripts", 1).keeping(1).costing(4).weighing(1.5),
                Step::new("waves", 4).keeping(1),
                Step::new("filmstrips", 4).keeping(1).costing(2),
            ],
            8,
        )
        .unwrap()
    }

    /// Ticks a second apart from `from`, each step doing `speed` blocks a second per place.
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
                c.work(codes[i], left[i], done[i]);
            }
            c.tick(from + s as f64);
        }
    }

    #[test]
    fn every_step_with_work_holds_a_place_and_the_rest_of_the_budget_goes_to_what_ends_last() {
        let mut c = ingest();
        c.work("transcripts", 100, 0);
        c.work("waves", 100, 0);
        c.work("filmstrips", 100, 0);
        c.busy(0.5);
        let decision = c.tick(0.0).unwrap();
        assert_eq!(decision.budget, 8);
        assert!(c.places("transcripts") == 1 && c.places("waves") >= 1 && c.places("filmstrips") >= 1, "{decision:?}");
        // The filmstrips go slow, the waves fast: the filmstrips get the places.
        let (mut left, mut done) = ([100, 100, 100], [0, 0, 0]);
        run(&mut c, 1.0, 30, &mut left, &mut done, [0.5, 2.0, 0.2]);
        assert!(c.places("filmstrips") > c.places("waves"), "transcripts {} waves {} filmstrips {}", c.places("transcripts"), c.places("waves"), c.places("filmstrips"));
    }

    #[test]
    fn the_budget_shrinks_while_the_processor_is_full_and_never_below_what_the_steps_keep() {
        let mut c = ingest();
        c.work("transcripts", 10, 0);
        c.work("waves", 10, 0);
        c.work("filmstrips", 10, 0);
        c.busy(1.0);
        for s in 0..10 {
            c.tick(s as f64 * PERIOD_SECONDS);
        }
        let decision = c.tick(1000.0).unwrap();
        assert_eq!(decision.budget, 4 + 1 + 2, "the places kept: transcription 4, waves 1, filmstrips 2");
        assert_eq!((c.places("transcripts"), c.places("waves"), c.places("filmstrips")), (1, 1, 1));
    }

    #[test]
    fn a_step_with_nothing_left_holds_no_place_at_once() {
        let mut c = ingest();
        c.work("waves", 5, 0);
        c.busy(0.5);
        c.tick(0.0);
        assert!(c.places("waves") >= 1);
        c.work("waves", 0, 5);
        c.tick(1.0);
        assert_eq!(c.places("waves"), 0);
    }

    #[test]
    fn a_step_twice_is_refused_and_an_unknown_step_has_no_place() {
        assert!(Conductor::new(vec![Step::new("waves", 1), Step::new("waves", 2)], 8).is_err());
        assert_eq!(ingest().places("teleport"), 0);
    }
}
