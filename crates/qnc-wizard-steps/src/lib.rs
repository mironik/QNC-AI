//! The steps of the QNC AI wizard and the one rule of going through them. The steps are data
//! (`contracts/wizard-steps.json`); which are done is the project database's
//! (`qnc-wizard-db`), never this block's: it only says, from the done steps, which step is the
//! current one and which a user may open. A step is open when every step before it is done; Next
//! needs the shown step done; Back is always there. It reads and writes nothing.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use serde::Deserialize;

pub const STEPS: &str = include_str!("../../../contracts/wizard-steps.json");

/// One step: its code, its name and when it is done (texts of the shown language).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub code: String,
    pub name: String,
    pub done_when: String,
}

#[derive(Deserialize)]
struct File {
    #[allow(dead_code)]
    note: String,
    steps: Vec<Step>,
}

/// The steps of the wizard in their order; a code twice or no steps is an error.
pub fn parse(json: &str) -> Result<Vec<Step>, String> {
    let mut value: serde_json::Value = serde_json::from_str(json).map_err(|error| format!("wizard steps: {error}"))?;
    qnc_wizard_text::resolve(&mut value);
    let file: File = serde_json::from_value(value).map_err(|error| format!("wizard steps: {error}"))?;
    let mut codes = BTreeSet::new();
    for step in &file.steps {
        if step.code.is_empty() || !codes.insert(step.code.as_str()) {
            return Err(format!("wizard steps: '{}' is empty or twice", step.code));
        }
    }
    if file.steps.is_empty() {
        return Err("wizard steps: none".into());
    }
    Ok(file.steps)
}

/// The steps of the wizard.
pub fn steps() -> &'static [Step] {
    static STEPS_READ: OnceLock<Vec<Step>> = OnceLock::new();
    STEPS_READ.get_or_init(|| parse(STEPS).expect("the wizard's steps"))
}

/// Whether `code` is a step of the wizard.
pub fn is_step(code: &str) -> bool {
    steps().iter().any(|step| step.code == code)
}

/// Where a user is, from the steps that are done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    done: Vec<bool>,
}

impl Progress {
    pub fn new(steps: &[Step], done: &BTreeSet<String>) -> Self {
        Self { done: steps.iter().map(|step| done.contains(&step.code)).collect() }
    }

    pub fn len(&self) -> usize {
        self.done.len()
    }

    pub fn is_empty(&self) -> bool {
        self.done.is_empty()
    }

    pub fn is_done(&self, index: usize) -> bool {
        self.done.get(index).copied().unwrap_or(false)
    }

    /// The first step not done (the last when every step is done).
    pub fn current(&self) -> usize {
        self.done.iter().position(|done| !done).unwrap_or(self.done.len().saturating_sub(1))
    }

    /// A step may be opened when every step before it is done.
    pub fn may_open(&self, index: usize) -> bool {
        index < self.done.len() && self.done[..index].iter().all(|done| *done)
    }

    /// Next from the shown step: only when it is done and a next step exists.
    pub fn next(&self, shown: usize) -> Option<usize> {
        (self.is_done(shown) && shown + 1 < self.done.len()).then_some(shown + 1)
    }

    /// Back from the shown step.
    pub fn back(&self, shown: usize) -> Option<usize> {
        shown.checked_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn done(codes: &[&str]) -> BTreeSet<String> {
        codes.iter().map(|code| code.to_string()).collect()
    }

    #[test]
    fn the_steps_of_the_wizard_are_read_with_their_names() {
        let steps = steps();
        assert_eq!(steps.len(), 10);
        assert_eq!(steps[0].code, "new_story");
        assert_eq!(steps[0].name, "New story");
        assert!(is_step("b_roll") && !is_step("auto_b_roll"));
    }

    #[test]
    fn a_step_opens_after_every_step_before_it_is_done_and_next_needs_the_shown_one_done() {
        let steps = steps();
        let fresh = Progress::new(steps, &done(&[]));
        assert_eq!(fresh.current(), 0);
        assert!(fresh.may_open(0) && !fresh.may_open(1));
        assert_eq!(fresh.next(0), None);
        assert_eq!(fresh.back(0), None);

        let two = Progress::new(steps, &done(&["new_story", "import"]));
        assert_eq!(two.current(), 2);
        assert!(two.may_open(2) && !two.may_open(3));
        assert_eq!(two.next(1), Some(2));
        assert_eq!(two.back(2), Some(1));

        let reopened = Progress::new(steps, &done(&["new_story", "statements"]));
        assert_eq!(reopened.current(), 1, "a step opened again is where the user is");
        assert!(!reopened.may_open(2), "a step after a step not done stays locked");

        let all: Vec<&str> = steps.iter().map(|step| step.code.as_str()).collect();
        let finished = Progress::new(steps, &done(&all));
        assert_eq!(finished.current(), 9);
        assert_eq!(finished.next(9), None);
    }

    #[test]
    fn a_step_twice_is_refused() {
        let json = r#"{"note":"","steps":[{"code":"a","name":"A","done_when":"x"},{"code":"a","name":"B","done_when":"y"}]}"#;
        assert!(parse(json).is_err());
    }
}
