//! The order of the steps of an ingest (user 2026-11-16: "redosljed bi trebao moći biti
//! variabilan i trebao bi ući u postavke templatea"; "promjenom orkestratora mjenjaju i redosljed
//! i mjesto spremanja"). The blocks of the ingest only do their work; an orchestrator reads this
//! plan from the settings of the project (the template's `artifacts.order`) and runs them in
//! its stages: a stage after the one before it, the steps of one stage together.
//!
//! The plan is the template's: a project without it is a controlled error, never a default
//! substituted; [`DEFAULT`] is the baseline the system templates are seeded with.

use serde_json::Value;

/// One step of an ingest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Step {
    /// The card records completed (a probe of what a record lacks).
    Records,
    /// The posters of the clips without one.
    Posters,
    /// The first AI look, from the posters.
    PosterLook,
    Waves,
    Filmstrips,
    /// The clips copied or linked by the project's settings.
    Import,
    /// The transcription of the speech channel.
    Transcripts,
    /// The analysis of the shots.
    Shots,
    /// What the shots show (AI).
    Content,
    /// The sentences of what the shots show (AI).
    Captions,
}

impl Step {
    pub const ALL: [Step; 10] = [
        Step::Records,
        Step::Posters,
        Step::PosterLook,
        Step::Waves,
        Step::Filmstrips,
        Step::Import,
        Step::Transcripts,
        Step::Shots,
        Step::Content,
        Step::Captions,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Step::Records => "records",
            Step::Posters => "posters",
            Step::PosterLook => "poster_look",
            Step::Waves => "waves",
            Step::Filmstrips => "filmstrips",
            Step::Import => "import",
            Step::Transcripts => "transcripts",
            Step::Shots => "shots",
            Step::Content => "content",
            Step::Captions => "captions",
        }
    }

    pub fn of(code: &str) -> Option<Step> {
        Step::ALL.into_iter().find(|step| step.code() == code)
    }

    /// The steps whose results this one works from: they must be in an earlier stage or the
    /// same one.
    pub fn needs(self) -> &'static [Step] {
        match self {
            Step::Waves | Step::Filmstrips | Step::Shots => &[Step::Records],
            Step::PosterLook => &[Step::Posters],
            Step::Content => &[Step::Shots],
            Step::Captions => &[Step::Content],
            _ => &[],
        }
    }
}

/// The stages of an ingest, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub stages: Vec<Vec<Step>>,
}

/// The baseline of the system templates: the order the worker had (2026-11-16).
pub const DEFAULT: &str = r#"[["records"], ["posters", "poster_look"], ["waves", "filmstrips", "import", "transcripts"], ["shots", "content", "captions"]]"#;

impl Plan {
    /// The plan in `artifacts` of the project's settings (its `order`).
    pub fn of_settings(artifacts: &Value) -> Result<Plan, String> {
        let order = artifacts.get("order").ok_or("the project has no order of its ingest steps (artifacts.order)")?;
        Plan::of_value(order)
    }

    pub fn of_value(order: &Value) -> Result<Plan, String> {
        let stages = order.as_array().ok_or("artifacts.order is not a list of stages")?;
        let mut plan = Plan { stages: Vec::new() };
        for stage in stages {
            let codes = stage.as_array().ok_or("a stage of artifacts.order is not a list of steps")?;
            let mut steps = Vec::new();
            for code in codes {
                let code = code.as_str().ok_or("a step of artifacts.order is not a name")?;
                let step = Step::of(code).ok_or_else(|| format!("artifacts.order: no step {code}"))?;
                if plan.has(step) || steps.contains(&step) {
                    return Err(format!("artifacts.order: {code} twice"));
                }
                steps.push(step);
            }
            if steps.is_empty() {
                return Err("artifacts.order: an empty stage".into());
            }
            plan.stages.push(steps);
        }
        plan.check()?;
        Ok(plan)
    }

    pub fn default_plan() -> Plan {
        Plan::of_value(&serde_json::from_str(DEFAULT).expect("the default order is JSON")).expect("the default order is a plan")
    }

    /// Whether the plan has `step` at all (a template may leave a step out).
    pub fn has(&self, step: Step) -> bool {
        self.stage_of(step).is_some()
    }

    /// The stage `step` is in.
    pub fn stage_of(&self, step: Step) -> Option<usize> {
        self.stages.iter().position(|stage| stage.contains(&step))
    }

    /// Every step after the steps it works from.
    fn check(&self) -> Result<(), String> {
        for (at, stage) in self.stages.iter().enumerate() {
            for step in stage {
                for need in step.needs() {
                    match self.stage_of(*need) {
                        Some(need_at) if need_at <= at => {}
                        Some(_) => return Err(format!("artifacts.order: {} comes before {}", step.code(), need.code())),
                        None => return Err(format!("artifacts.order: {} needs {}", step.code(), need.code())),
                    }
                }
            }
        }
        Ok(())
    }

    /// The plan as the setting writes it.
    pub fn to_value(&self) -> Value {
        Value::Array(self.stages.iter().map(|stage| Value::Array(stage.iter().map(|step| Value::String(step.code().into())).collect())).collect())
    }

    /// One line for the log.
    pub fn line(&self) -> String {
        self.stages.iter().map(|stage| stage.iter().map(|step| step.code()).collect::<Vec<_>>().join("+")).collect::<Vec<_>>().join(" > ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_default_order_is_a_plan_and_written_back_the_same() {
        let plan = Plan::default_plan();
        assert_eq!(plan.stages.len(), 4);
        assert_eq!(Plan::of_value(&plan.to_value()).unwrap(), plan);
        assert_eq!(plan.line(), "records > posters+poster_look > waves+filmstrips+import+transcripts > shots+content+captions");
    }

    #[test]
    fn a_step_before_what_it_works_from_or_twice_or_unknown_is_refused() {
        assert!(Plan::of_value(&json!([["waves"], ["records"]])).is_err());
        assert!(Plan::of_value(&json!([["records", "waves", "records"]])).is_err());
        assert!(Plan::of_value(&json!([["records"], ["teleport"]])).is_err());
        assert!(Plan::of_value(&json!([["content"]])).is_err(), "content without shots");
        let transcripts_first = Plan::of_value(&json!([["transcripts"], ["records", "waves"]])).unwrap();
        assert_eq!(transcripts_first.stage_of(Step::Transcripts), Some(0));
        assert!(!transcripts_first.has(Step::Filmstrips), "a template may leave a step out");
    }

    #[test]
    fn a_project_without_its_order_is_an_error_not_a_default() {
        assert!(Plan::of_settings(&json!({"filmstrip": "auto"})).is_err());
        assert!(Plan::of_settings(&json!({"order": serde_json::from_str::<Value>(DEFAULT).unwrap()})).is_ok());
    }
}
