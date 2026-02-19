use crate::weight_sampler::WeightSampler;
use libprotocol::schema::{Journey, Scenario};

#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub scenario_name: String,
    pub version: String,
    pub base_url: String,
    pub journeys: Vec<libprotocol::schema::Journey>,
    pub weight_sampler: WeightSampler,
    pub limits: Option<String>,
}

impl From<&Scenario> for ExecutionPlan {
    fn from(scenario: &Scenario) -> Self {

        Self {
            scenario_name: scenario.name.to_string(),
            version: scenario.version.to_string(),
            base_url: scenario.target.base_url.to_string(),
            journeys: scenario.journeys.clone(),
            weight_sampler: WeightSampler::from(scenario),
            limits: None,
        }
    }
}

impl ExecutionPlan {
    pub fn get_journey(&self, id: i32) -> &Journey {
        self.journeys.get(id as usize).unwrap()
    }
}

pub(crate) fn calculate_cumulative_ends(weights: &mut [u16]) -> Vec<u32> {
    let mut ends = Vec::with_capacity(weights.len());
    let mut acc: u32 = 0;
    for &mut w in weights {
        // лучше валидировать, что w > 0
        acc += w as u32;
        ends.push(acc);
    }
    ends
}

#[cfg(test)]
mod tests {
    use crate::execution_plan::{calculate_cumulative_ends, ExecutionPlan};
    use libprotocol::Scenario;
    use std::path::PathBuf;

    #[test]
    fn it_check_calculate_cumulative_ends() {
        let result = calculate_cumulative_ends(&mut Vec::from([7, 3]));

        assert_eq!(Vec::from([7, 10]), result);
    }

    #[test]
    fn it_create_execution_plan_for_scenario() {
        let path = fixture_path("weight-sampler-test.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let execution_plan = ExecutionPlan::from(&scenario);
        insta::assert_debug_snapshot!(execution_plan);
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}