use std::collections::{BTreeMap, HashMap};
use std::fmt::format;
use std::ops::Range;
use libprotocol::schema::Step;
use serde::{Deserialize, Serialize};
use crate::execution_plan::ExecutionPlan;

#[derive(Debug, Serialize, Deserialize)]
pub struct StepsCounting {
    pub request_count: i32,
    pub sleep_count: i32,
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

pub fn dry_run(mut plan: ExecutionPlan, iterations: u32, seed: u32) -> DryRunReport {

    let mut report = DryRunReport {
        iterations,
        seed,
        journeys: 0,
        steps: StepsCounting{ request_count: 0, sleep_count: 0 },
        endpoints: BTreeMap::new()
    };

    let mut sampler = &mut plan.weight_sampler;
    sampler.seed = seed.to_string();

    for i in 1..=iterations {
        run_plan(&plan, seed, &mut report, i);
    }


    report
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
    use std::path::PathBuf;
    use libprotocol::Scenario;
    use crate::dry_runner::dry_run;
    use crate::execution_plan::ExecutionPlan;

    #[test]
    fn dry_run_scenario() {
        let scenario: &Scenario = &libprotocol::parse_scenario(fixture_path("valid-extended-scenario.json"));
        let plan = ExecutionPlan::from(scenario);
        let report = dry_run(plan, 100, 12345);
       insta::assert_debug_snapshot!(report)
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}