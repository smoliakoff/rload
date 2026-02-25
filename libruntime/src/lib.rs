pub mod execution_plan;
mod weight_sampler;
mod dry_runner;
pub mod scheduler;
mod vu_runner;
mod run_engine;
mod metrics;
pub mod events;

use crate::execution_plan::ExecutionPlan;
pub use libprotocol::schema::Journey;
use libprotocol::Scenario;
use std::path::Path;
use std::sync::Arc;
use crate::dry_runner::DryRunMode;
use crate::events::{Event, EventSink};

pub struct AppContext {
    pub scenario: Arc<Scenario>
}

pub fn plan(scenario: &Scenario, seed: u32) -> ExecutionPlan {
    let mut plan = ExecutionPlan::from(scenario);
    plan.weight_sampler.seed = seed.to_string();

    plan
}
pub async fn dry_run(scenario_path: impl AsRef<Path>, seed: u32, iterations: u32, is_simulated: bool, sink: EventSink<Event>) {

    let scenario: &Scenario = &libprotocol::parse_scenario(&scenario_path);
    let mode = match is_simulated {
        true => DryRunMode::Simulated(scenario),
        false => DryRunMode::PlanOnly
    };
    libprotocol::validate(&scenario_path).expect("scenario must be valid");
    let report = dry_runner::dry_run(ExecutionPlan::from(scenario), iterations, seed, mode, sink).await;

    println!("{:?}", report)
}

pub async fn run(scenario_path: impl AsRef<Path>, is_mock: Option<bool>, sink: EventSink<Event>) {
    let scenario = &libprotocol::parse_scenario(&scenario_path);
    let execution_plan: ExecutionPlan = ExecutionPlan::from(scenario);

    let report = run_engine::RunEngine::new(Some(is_mock.unwrap()), Some(true))
        .run(&execution_plan, scenario, sink).await;

    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn it_works_run_mock() {
        let fixture = fixture_path("valid-extended-scenario.json");
        let scenario = libprotocol::parse_scenario(&fixture);
        let mut counts: HashMap<i32, u32> = HashMap::new();
        let plan = plan(&scenario, 100000);
        let attempts = 300_000;
        let sampler = &plan.weight_sampler;
        for i in 0..attempts {
            let stable_key = format!("scenario={}:index={}", plan.scenario_name, i);

            let journey_id = sampler.peek(&stable_key).unwrap();
            *counts.entry(journey_id).or_insert(0) += 1;
        }

        println!("{:#?}", &counts);

        fn sampler_weight_for(i: i32, plan: &ExecutionPlan) -> i32 {
            plan.journeys.get(i as usize)
                .expect("invalid journey_id")
                .weight as i32
        }

        fn sampler_name_for(i: i32, plan: &ExecutionPlan) -> &str {
            plan.journeys.get(i as usize)
                .expect("invalid journey_id")
                .name.as_str()
        }

        let tolerance = 0.02; // 2%

        for count in counts.iter() {
            let (i, val) = count;
            let expected_ratio =
                sampler_weight_for(*i, &plan) as f64 / sampler.total_weight as f64;

            let actual_ratio = *val as f64 / attempts as f64;

            let diff = (expected_ratio - actual_ratio).abs();

            assert!(
                diff < tolerance,
                "journey {}: expected {:.4}, got {:.4}",
                i,
                expected_ratio,
                actual_ratio
            );

            println!("journey {}: expected {:.4}, got {:.4}",
                     sampler_name_for(*i, &plan),
                     expected_ratio,
                     actual_ratio)
        }
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}
