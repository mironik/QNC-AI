//! The conductor of an ingest (user 2026-11-17: "bitno je da svi završe približno u isto vrijeme;
//! ako je val brz, oduzmi mu vrijeme; daj kome treba"). It does not foresee how long anything
//! takes: it looks at how much of its work each step has done and gives the processor to the one
//! behind.
//!
//! The steps work in blocks (a clip, a piece of speech) and ask before each block how many they
//! may work on at once (their places). Every period each step tells its work done and its work in
//! all (its own unit: seconds of speech heard, clips made), and whether it has a block to take
//! now. The places are given anew:
//! - every step with work holds the places it keeps (the transcription and the filmstrips one: a
//!   stop and a start cost them more than it gives; the waves none: fast, they may wait);
//! - the rest of the processor's threads (a place costs the threads it uses) goes one place at a
//!   time to the step with the smallest share of its work done by the next look, so the shares
//!   draw level and the processor stays full.
//!
//! It only decides; it reads nothing, starts nothing and keeps nothing between runs. The clock is
//! given (seconds), so the rule is tested without waiting.

use std::collections::VecDeque;

/// How often the places are given anew.
pub const PERIOD_SECONDS: f64 = 2.0;
/// How far ahead the shares done are foreseen when the places are given.
pub const HORIZON_SECONDS: f64 = 10.0;
/// Over how long a place's speed is measured.
pub const WINDOW_SECONDS: f64 = 30.0;
/// Share of its work a place does a second until its own speed is measured.
pub const FIRST_SPEED: f64 = 0.002;

/// One step: its code, how many blocks it can work on at once at most, how many it keeps with
/// work, what a place costs of the processor (threads), and whether its share is kept level.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub code: String,
    pub most: usize,
    pub least: usize,
    pub cost: usize,
    pub balanced: bool,
}

impl Step {
    pub fn new(code: &str, most: usize) -> Self {
        Self { code: code.to_string(), most: most.max(1), least: 0, cost: 1, balanced: false }
    }

    /// With work it holds `least` places at least.
    pub fn keeping(mut self, least: usize) -> Self {
        self.least = least.min(self.most);
        self
    }

    /// A place costs `cost` threads of the processor.
    pub fn costing(mut self, cost: usize) -> Self {
        self.cost = cost.max(1);
        self
    }

    /// Its share of work done is kept level with the other balanced steps.
    pub fn balanced(mut self) -> Self {
        self.balanced = true;
        self
    }
}

/// What a period decided, for the log: each step with its places, the clips it has left and the
/// clips it started with (user 2026-11-17: "brojač obrnuto, broji koliko je ostalo").
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub steps: Vec<(String, usize, usize, usize)>,
}

#[derive(Debug, Clone)]
struct State {
    step: Step,
    done: f64,
    all: f64,
    can: bool,
    places: usize,
    held: f64,
    /// (seconds, work done, place-seconds held) over the window.
    samples: VecDeque<(f64, f64, f64)>,
}

impl State {
    fn share(&self) -> f64 {
        if self.all > 0.0 {
            (self.done / self.all).min(1.0)
        } else {
            1.0
        }
    }

    /// Share of its work one place does a second, measured.
    fn speed(&self) -> Option<f64> {
        let (first, last) = (self.samples.front()?, self.samples.back()?);
        let held = last.2 - first.2;
        let done = last.1 - first.1;
        (held >= 2.0 && done > 0.0 && self.all > 0.0).then(|| done / self.all / held)
    }

    fn foreseen(&self, places: usize) -> f64 {
        self.share() + self.speed().unwrap_or(FIRST_SPEED) * places as f64 * HORIZON_SECONDS
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
            steps: steps.into_iter().map(|step| State { step, done: 0.0, all: 0.0, can: false, places: 0, held: 0.0, samples: VecDeque::new() }).collect(),
            threads: threads.max(1),
            given: f64::NEG_INFINITY,
            last: 0.0,
        })
    }

    fn at(&self, code: &str) -> Option<usize> {
        self.steps.iter().position(|state| state.step.code == code)
    }

    /// `code` has done `done` of its `all` work (its own unit) and `can` take a block now.
    pub fn work(&mut self, code: &str, done: f64, all: f64, can: bool) {
        if let Some(i) = self.at(code) {
            let now = self.last;
            let state = &mut self.steps[i];
            (state.done, state.all, state.can) = (done, all, can);
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
        let mut places: Vec<usize> = self
            .steps
            .iter()
            .map(|state| match (state.can, state.step.balanced) {
                (false, _) => 0,
                (true, true) => state.step.least,
                // Not balanced (the AI, held back by the worker until the others are done).
                (true, false) => state.step.most,
            })
            .collect();
        let mut spent: usize = places.iter().zip(&self.steps).map(|(p, state)| p * state.step.cost).sum();
        loop {
            let next = (0..self.steps.len())
                .filter(|&i| {
                    let state = &self.steps[i];
                    state.can && state.step.balanced && places[i] < state.step.most && spent + state.step.cost <= self.threads
                })
                .min_by(|&a, &b| self.steps[a].foreseen(places[a]).total_cmp(&self.steps[b].foreseen(places[b])));
            let Some(i) = next else { break };
            places[i] += 1;
            spent += self.steps[i].step.cost;
        }
        for (state, places) in self.steps.iter_mut().zip(places) {
            state.places = places;
        }
        Some(Decision { steps: self.steps.iter().map(|state| (state.step.code.clone(), state.places, (state.all - state.done).max(0.0).round() as usize, state.all.round() as usize)).collect() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest() -> Conductor {
        Conductor::new(
            vec![
                Step::new("transcripts", 3).keeping(1).costing(2).balanced(),
                Step::new("waves", 4).balanced(),
                Step::new("filmstrips", 8).keeping(1).balanced(),
                Step::new("shots", 1),
            ],
            8,
        )
        .unwrap()
    }

    /// `seconds` ticks a second apart; each step does `speed` units a second per place, of `all`.
    fn run(c: &mut Conductor, seconds: usize, done: &mut [f64; 3], all: [f64; 3], speed: [f64; 3]) {
        let codes = ["transcripts", "waves", "filmstrips"];
        for s in 0..seconds {
            for i in 0..3 {
                done[i] = (done[i] + speed[i] * c.places(codes[i]) as f64).min(all[i]);
                c.work(codes[i], done[i], all[i], done[i] < all[i]);
            }
            c.tick(s as f64);
        }
    }

    #[test]
    fn the_shares_done_draw_level_and_the_fast_waves_give_their_time() {
        let mut c = ingest();
        // 1850 s of speech, 103 clips; a transcription place hears 5 s a second, a wave place
        // makes 1.5 clips a second, a filmstrip place 0.4.
        let (all, mut done) = ([1850.0, 103.0, 103.0], [0.0; 3]);
        run(&mut c, 60, &mut done, all, [5.0, 1.5, 0.4]);
        let shares: Vec<f64> = (0..3).map(|i| done[i] / all[i]).collect();
        let (most, least) = (shares.iter().cloned().fold(0.0, f64::max), shares.iter().cloned().fold(1.0, f64::min));
        assert!(most - least < 0.1, "shares {shares:?}");
        assert_eq!(c.places("transcripts"), 3, "the costly transcription takes most of the processor");
        assert!(c.places("waves") <= 1, "the fast waves wait");
    }

    #[test]
    fn a_step_that_cannot_take_a_block_holds_no_place() {
        let mut c = ingest();
        c.work("transcripts", 10.0, 100.0, true);
        c.work("waves", 50.0, 50.0, false);
        c.work("filmstrips", 10.0, 100.0, true);
        c.tick(0.0);
        assert_eq!(c.places("waves"), 0);
        assert!(c.places("transcripts") >= 1 && c.places("filmstrips") >= 1);
    }

    #[test]
    fn a_step_not_balanced_takes_all_its_places_with_work() {
        let mut c = ingest();
        c.work("shots", 0.0, 5.0, true);
        c.tick(0.0);
        assert_eq!(c.places("shots"), 1);
    }

    #[test]
    fn a_step_twice_is_refused_and_an_unknown_step_has_no_place() {
        assert!(Conductor::new(vec![Step::new("waves", 1), Step::new("waves", 2)], 8).is_err());
        assert_eq!(ingest().places("teleport"), 0);
    }
}
