//! The conductor of an ingest (user 2026-11-17: "trebaš tražiti drugačiju organizaciju procesa i
//! optimizirati dinamičkim prilagođavanjem"). The steps of the background work (the copy, the
//! transcription, the waves, the filmstrips, the AI) no longer decide for themselves when they may
//! work: each asks the conductor how many clips it may work on at once (its places), and tells it
//! how much it has waiting, how many it works on, and how fast it goes.
//!
//! The rule, read every tick:
//! - The steps have an order of priority (data, the template's); the copy first, it feeds the rest.
//! - The highest step that has work (the lead) gets all its places.
//! - A lower step gets a place only by a probe: when nothing changed for a while and the processor
//!   has room, the highest step below the lead that could use one more place gets it, and the rate
//!   every step above it had then is kept as what it must not lose.
//! - When a step above goes slower than that by more than the margin, the lowest step holding a
//!   place gives one back.
//! - A step with nothing waiting and nothing running holds no place: the next one leads.
//!
//! It only decides; it reads nothing, starts nothing and keeps nothing between runs. The clock is
//! given (seconds), so the rule is tested without waiting.

/// How long nothing must change before a lower step is given one more place.
pub const PROBE_SECONDS: f64 = 5.0;
/// How much slower than its kept rate a higher step may go before a lower step gives a place back.
pub const SLOWER_BY: f64 = 0.10;
/// Above this share of the processor busy no place is added by a probe.
pub const BUSY: f64 = 0.90;

/// One step: its code and how many clips it can work on at once at most.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub code: String,
    pub most: usize,
}

impl Step {
    pub fn new(code: &str, most: usize) -> Self {
        Self { code: code.to_string(), most: most.max(1) }
    }
}

/// What a tick decided, for the log.
#[derive(Debug, Clone, PartialEq)]
pub enum Decision {
    /// `step` leads (the highest with work): all its places.
    Leads { step: String, places: usize },
    /// `step` got one more place by a probe; `kept` the rates of the steps above it then.
    Added { step: String, places: usize, kept: Vec<(String, f64)> },
    /// `step` gave a place back: `slower` went below its kept rate.
    GaveBack { step: String, places: usize, slower: String, rate: f64, kept: f64 },
    /// `step` has nothing to do: no place.
    Idle { step: String },
}

#[derive(Debug, Clone)]
struct State {
    step: Step,
    waiting: usize,
    running: usize,
    rate: Option<f64>,
    kept: Option<f64>,
    places: usize,
}

impl State {
    fn has_work(&self) -> bool {
        self.waiting > 0 || self.running > 0
    }
}

/// The conductor of one run.
#[derive(Debug, Clone)]
pub struct Conductor {
    steps: Vec<State>,
    busy: Option<f64>,
    changed: f64,
    /// The step that led the tick before.
    lead: Option<usize>,
}

impl Conductor {
    /// The steps in their order of priority, the first the highest; a code twice is an error.
    pub fn new(steps: Vec<Step>) -> Result<Self, String> {
        let mut seen = std::collections::BTreeSet::new();
        for step in &steps {
            if !seen.insert(step.code.clone()) {
                return Err(format!("conductor: step {} twice", step.code));
            }
        }
        Ok(Self { steps: steps.into_iter().map(|step| State { step, waiting: 0, running: 0, rate: None, kept: None, places: 0 }).collect(), busy: None, changed: f64::NEG_INFINITY, lead: None })
    }

    fn at(&self, code: &str) -> Option<usize> {
        self.steps.iter().position(|state| state.step.code == code)
    }

    /// How many clips `code` has waiting and works on now.
    pub fn work(&mut self, code: &str, waiting: usize, running: usize) {
        if let Some(i) = self.at(code) {
            (self.steps[i].waiting, self.steps[i].running) = (waiting, running);
        }
    }

    /// How fast `code` goes (its own unit a second: MB of the copy, seconds of speech heard).
    pub fn rate(&mut self, code: &str, rate: f64) {
        if let Some(i) = self.at(code) {
            self.steps[i].rate = Some(rate);
        }
    }

    /// The share of the processor busy (0..1).
    pub fn busy(&mut self, share: f64) {
        self.busy = Some(share.clamp(0.0, 1.0));
    }

    /// How many clips `code` may work on at once now (a step the conductor does not know: none).
    pub fn places(&self, code: &str) -> usize {
        self.at(code).map_or(0, |i| self.steps[i].places)
    }

    /// One tick at `now` (seconds): the places given again; what changed.
    pub fn tick(&mut self, now: f64) -> Vec<Decision> {
        let mut said = Vec::new();
        // No work, no place.
        for state in &mut self.steps {
            if !state.has_work() && state.places > 0 {
                state.places = 0;
                state.kept = None;
                said.push(Decision::Idle { step: state.step.code.clone() });
            }
        }
        let Some(lead) = self.steps.iter().position(State::has_work) else { return said };
        // A higher step leads now: the steps below it hold what they had while it had nothing,
        // so they hold nothing and earn their places again by a probe (2026-11-17 log: the
        // filmstrips kept four places when the transcription began).
        if self.lead.is_none_or(|was| lead < was) {
            for state in &mut self.steps[lead + 1..] {
                if state.places > 0 {
                    state.places = 0;
                    state.kept = None;
                    said.push(Decision::Idle { step: state.step.code.clone() });
                    self.changed = now;
                }
            }
        }
        self.lead = Some(lead);
        if self.steps[lead].places != self.steps[lead].step.most {
            self.steps[lead].places = self.steps[lead].step.most;
            // The lead has no step above it to keep a rate for.
            self.steps[lead].kept = None;
            said.push(Decision::Leads { step: self.steps[lead].step.code.clone(), places: self.steps[lead].places });
        }
        // A step above that went slower than its kept rate: the lowest step holding a place below
        // it gives one back.
        let slower = self.steps.iter().enumerate().find_map(|(i, state)| match (state.rate, state.kept) {
            (Some(rate), Some(kept)) if state.has_work() && rate < kept * (1.0 - SLOWER_BY) => Some((i, rate, kept)),
            _ => None,
        });
        if let Some((above, rate, kept)) = slower {
            if let Some(low) = (above + 1..self.steps.len()).rev().find(|&i| i != lead && self.steps[i].places > 0) {
                self.steps[low].places -= 1;
                // What it must not lose is now what it does.
                self.steps[above].kept = Some(rate);
                self.changed = now;
                said.push(Decision::GaveBack { step: self.steps[low].step.code.clone(), places: self.steps[low].places, slower: self.steps[above].step.code.clone(), rate, kept });
                return said;
            }
        }
        // A probe: nothing changed for a while and the processor has room.
        let room = self.busy.is_none_or(|busy| busy < BUSY);
        if room && now - self.changed >= PROBE_SECONDS {
            if let Some(next) = (lead + 1..self.steps.len()).find(|&i| self.steps[i].has_work() && self.steps[i].places < self.steps[i].step.most) {
                self.steps[next].places += 1;
                let mut kept = Vec::new();
                for state in &mut self.steps[..next] {
                    if let Some(rate) = state.rate.filter(|_| state.has_work()) {
                        state.kept = Some(rate);
                        kept.push((state.step.code.clone(), rate));
                    }
                }
                self.changed = now;
                said.push(Decision::Added { step: self.steps[next].step.code.clone(), places: self.steps[next].places, kept });
            }
        }
        said
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ingest() -> Conductor {
        Conductor::new(vec![Step::new("import", 1), Step::new("transcripts", 2), Step::new("waves", 4), Step::new("filmstrips", 4)]).unwrap()
    }

    #[test]
    fn the_highest_step_with_work_leads_with_all_its_places_and_the_rest_wait_for_a_probe() {
        let mut c = ingest();
        c.work("import", 100, 1);
        c.work("transcripts", 0, 0);
        c.work("waves", 3, 0);
        c.busy(0.95);
        c.tick(0.0);
        assert_eq!((c.places("import"), c.places("waves")), (1, 0), "the processor is busy: nothing added");
        c.busy(0.5);
        c.tick(1.0);
        assert_eq!(c.places("waves"), 1, "room and nothing changed before: the waves get a place");
        c.tick(2.0);
        assert_eq!(c.places("waves"), 1, "the next probe waits");
        c.tick(6.5);
        assert_eq!(c.places("waves"), 2);
    }

    #[test]
    fn a_higher_step_that_goes_slower_takes_a_place_back_from_the_lowest() {
        let mut c = ingest();
        c.work("import", 100, 1);
        c.work("waves", 10, 0);
        c.work("filmstrips", 10, 0);
        c.busy(0.4);
        c.rate("import", 160.0);
        c.tick(0.0);
        assert_eq!(c.places("waves"), 1);
        c.tick(5.0);
        assert_eq!(c.places("waves"), 2);
        c.tick(10.0);
        c.tick(15.0);
        c.tick(20.0);
        assert_eq!((c.places("waves"), c.places("filmstrips")), (4, 1), "the waves full, then the filmstrips");
        // The copy slows from 160 to 120 MB/s: the filmstrips give their place back.
        c.rate("import", 120.0);
        let said = c.tick(21.0);
        assert!(matches!(&said[..], [Decision::GaveBack { step, slower, .. }] if step == "filmstrips" && slower == "import"));
        assert_eq!(c.places("filmstrips"), 0);
        // A rate within the margin is no reason.
        c.rate("import", 112.0);
        c.tick(22.0);
        assert_eq!(c.places("waves"), 4, "112 is within 10 % of the 120 kept after the give back");
    }

    #[test]
    fn a_step_with_nothing_to_do_holds_no_place_and_the_next_one_leads() {
        let mut c = ingest();
        c.work("import", 5, 1);
        c.work("transcripts", 5, 0);
        c.busy(0.99);
        c.tick(0.0);
        assert_eq!((c.places("import"), c.places("transcripts")), (1, 0));
        // The card is copied: the transcription leads with all its places at once.
        c.work("import", 0, 0);
        c.tick(1.0);
        assert_eq!((c.places("import"), c.places("transcripts")), (0, 2));
    }

    #[test]
    fn a_higher_step_that_begins_to_lead_takes_the_places_of_the_steps_below() {
        let mut c = ingest();
        c.work("filmstrips", 10, 0);
        c.busy(0.4);
        c.tick(0.0);
        assert_eq!(c.places("filmstrips"), 4, "alone, the filmstrips lead");
        c.work("transcripts", 5, 0);
        c.tick(1.0);
        assert_eq!((c.places("transcripts"), c.places("filmstrips")), (2, 0), "the transcription leads; the filmstrips earn their places again");
        c.tick(2.0);
        assert_eq!(c.places("filmstrips"), 0, "not before the next probe");
        c.tick(6.5);
        assert_eq!(c.places("filmstrips"), 1);
    }

    #[test]
    fn a_step_twice_is_refused_and_an_unknown_step_has_no_place() {
        assert!(Conductor::new(vec![Step::new("waves", 1), Step::new("waves", 2)]).is_err());
        assert_eq!(ingest().places("teleport"), 0);
    }
}
