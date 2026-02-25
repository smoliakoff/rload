use crate::execution_plan::ExecutionPlan;
use libprotocol::schema::Step;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use libprotocol::Scenario;
use crate::events::{Event, EventSink};
use crate::run_engine::{RunEngine, RunReport};

#[derive(Debug, Serialize, Deserialize)]
pub struct StepsCounting {
    pub request_count: i32,
    pub sleep_count: i32,
}
pub enum DryRunMode<'a> {
    PlanOnly,
    Simulated(&'a Scenario),
}

#[derive(Debug, Serialize)]
pub enum DryRunResult {
    PlanOnly(DryRunReport),
    Simulated(Box<RunReport>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DryRunReport {
    iterations: u32,
    seed: u32,
    /// journeys picked count total
    journeys: i32,
    steps: StepsCounting,
    endpoints: BTreeMap<String, i32>

}

pub async fn dry_run(plan: ExecutionPlan, iterations: u32, seed: u32, mode: DryRunMode<'_>, sink: EventSink<Event>) -> DryRunResult {

    match mode {
        DryRunMode::PlanOnly => {
            DryRunResult::PlanOnly(dry_run_plan_only(plan, iterations, seed))
        }
        DryRunMode::Simulated(scenario) => {
            DryRunResult::Simulated(Box::new(dry_run_simulated(plan, iterations, seed, scenario, sink).await))
        }
    }
}

pub fn dry_run_plan_only(mut plan: ExecutionPlan, iterations: u32, seed: u32) -> DryRunReport {

    let mut report = DryRunReport {
        iterations,
        seed,
        journeys: 0,
        steps: StepsCounting{ request_count: 0, sleep_count: 0 },
        endpoints: BTreeMap::new()
    };

    let sampler = &mut plan.weight_sampler;
    sampler.seed = seed.to_string();

    for i in 1..=iterations {
        run_plan(&plan, seed, &mut report, i);
    }


    report
}

pub async fn dry_run_simulated(plan: ExecutionPlan, _iterations: u32, _seed: u32, scenario: &Scenario, sink: EventSink<Event>) -> RunReport {
    RunEngine::new(Some(false), Some(false)).run(&plan, scenario, sink).await
}


fn run_plan(plan: &ExecutionPlan, seed: u32, report: &mut DryRunReport, iter_index: u32) {

    let journey = &plan.get_journey(
        plan.weight_sampler.peek(format!("{}-{}-{:?}", plan.scenario_name, seed, iter_index).as_str()).unwrap()
    );

    report.journeys += 1;

    for step in &journey.steps {
        match step {
            Step::Request{path, method: meth, ..} => {
                report.steps.request_count += 1;
                report.endpoints.entry(format!("{:?} {}", meth.clone(), path.clone())).and_modify(|count| *count += 1).or_insert(1);
            },
            Step::Sleep{..} => report.steps.sleep_count += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::dry_runner::{dry_run, DryRunMode};
    use crate::execution_plan::ExecutionPlan;
    use libprotocol::Scenario;
    use std::path::PathBuf;
    use crate::events::{Event, EventSink};

    #[tokio::test]
    async fn dry_run_scenario() {
        let scenario: &Scenario = &libprotocol::parse_scenario(fixture_path("valid-extended-scenario.json"));
        let plan = ExecutionPlan::from(scenario);
        let sink = EventSink::<Event>::noop();
        let report = dry_run(plan, 100, 12345, DryRunMode::PlanOnly, sink).await;
       insta::assert_debug_snapshot!(report)
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}