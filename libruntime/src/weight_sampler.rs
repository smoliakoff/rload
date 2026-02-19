use std::ptr::hash;
use sha2::{Digest, Sha256};
use libprotocol::Scenario;

#[derive(Debug, Clone)]
pub struct WeightSampler {
    pub seed: String,
    pub total_weight: u32,
    pub journey_ids: Vec<i32>,
    pub cumulative_ends: Vec<u32>
}

impl WeightSampler {
    pub fn new(scenario: Scenario, seed: String) -> Self {
        let mut ws = WeightSampler::from(&scenario);
        ws.seed = seed;
        ws
    }
    /// bucket должен быть в диапазоне 0..total_weight-1
    pub fn peek_bucket(&self, bucket: u32) -> Option<i32> {
        if self.total_weight == 0 || self.journey_ids.is_empty() {
            return None;
        }
        debug_assert!(bucket < self.total_weight);
        debug_assert_eq!(self.journey_ids.len(), self.cumulative_ends.len());

        // ищем первый end > bucket
        let idx = self.cumulative_ends
            .binary_search_by(|end| {
                if *end > bucket { std::cmp::Ordering::Greater } else { std::cmp::Ordering::Less }
            })
            .unwrap_or_else(|i| i);

        self.journey_ids.get(idx).copied()
    }

    /// полный вариант: сам строит bucket из stable_key
    pub fn peek(&self, stable_key: &str) -> Option<i32> {
        if self.total_weight == 0 {
            return None;
        }
        let key = format!("{stable_key}:seed={}", self.seed);
        let bucket = self.bucket_from_key(&key, self.total_weight as u32);
        self.peek_bucket(bucket)
    }

    fn bucket_from_key(&self, key: &str, total_weight: u32) -> u32 {
        debug_assert!(total_weight > 0);

        let digest = Sha256::digest(key.as_bytes()); // 32 bytes
        let first8: [u8; 8] = digest[0..8].try_into().unwrap();
        let n = u64::from_be_bytes(first8);

        (n % total_weight as u64) as u32
    }
}

impl From<&Scenario> for WeightSampler {
    fn from(scenario: &Scenario) -> Self {
        let total_weight: u16 = scenario.journeys.iter().map(|journey| journey.weight).sum();
        let journeys_ids = scenario.journeys.iter().enumerate()
            .map(|(i, _journey)| i as i32)
            .into_iter()
            .collect();
        let mut weights: Vec<u16> = scenario.journeys.iter().map(|journey| journey.weight).collect();
        WeightSampler {
            seed: "".to_string(),
            total_weight: total_weight.into(),
            journey_ids: journeys_ids,
            cumulative_ends: crate::execution_plan::calculate_cumulative_ends(&mut *weights),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::format;
    use std::path::PathBuf;
    use libprotocol::Scenario;
    use crate::weight_sampler::WeightSampler;

    #[test]
    fn it_calculate_weight_total_and_cumulative() {
        let path = fixture_path("weight-sampler-test.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let weight_sampler = WeightSampler::from(&scenario);
        insta::assert_debug_snapshot!(weight_sampler);
        assert_eq!(10, weight_sampler.total_weight);
    }

    #[test]
    fn it_peek_return_value_inside_range() {
        let path = fixture_path("weight-sampler-test.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let mut weight_sampler = WeightSampler::from(&scenario);
        weight_sampler.seed = "1000".to_string();
        for i in 0..10 {
            if let Some(res) = weight_sampler.peek(format!("{}-stable_key", i).as_str()){
                assert!(res >= 0 && res < 10);
            } else {
                panic!("Unexpected None result from peek");
            }
        }
    }

    #[test]
    fn it_check_peek_result_stable() {
        let path = fixture_path("weight-sampler-test.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let scenario: Scenario = serde_json::from_str(&content).unwrap();
        let weight_sampler = WeightSampler::from(&scenario);

        let mut result = Vec::with_capacity(10);
        for _ in 0..10 {
            result.push(weight_sampler.peek("stable_key"));
        }
        insta::assert_debug_snapshot!(result);
    }

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(name)
    }
}