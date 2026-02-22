use crate::schema::Step::Request;
use crate::ValidationError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::string::ToString;

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Scenario {
    pub  version: u16,
    pub  name: String,
    pub  target: Target,
    pub  workload: Workload,
    pub  journeys: Vec<Journey>,
    pub  description: Option<String>,
    pub  tags: Option<Vec<String>>,
    pub  thresholds: Option<Vec<Threshold>>,
    pub  metadata: Option<()>
}
impl Default for Scenario {
    fn default() -> Self {
        Self {
            version: 1,
            name: "default_scenario".to_string(),
            target: Target::default(),
            workload: Workload::default(),
            journeys: Vec::from([Journey::default()]),
            description: None,
            tags: None,
            thresholds: Option::from(vec![Threshold::default()]),
            metadata: None,
        }
    }
}
impl Scenario {
   pub fn set_version(mut self, version:u16) -> Self {
        self.version = version;
       self
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Target {
    pub  base_url: String,
    pub  default_headers: Option<BTreeMap<String, String>>,
    pub  insecure_tls: Option<bool>
}
impl Default for Target {
    fn default() -> Self {
        let mut headers: BTreeMap<String, String> = BTreeMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Self {
            base_url: "http://localhost:8080".parse().unwrap(),
            default_headers: Some(headers),
            insecure_tls: None,
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Workload {
    pub  stages: Vec<Stage>,
}
impl Default for Workload {
    fn default() -> Self {
        Self {
            stages: vec![Stage::default()],
        }
    }
}

impl Workload {
    pub fn get_rps_avg(&self) -> f64 {
        if self.stages.is_empty() {
            return 0.0;
        }

        self.stages
            .iter()
            .map(|stage| stage.rps.max(0) as f64)
            .sum::<f64>()
            / (self.stages.len() as f64)
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Stage {
    pub  duration_sec: i32,
    pub  rps: i32,
}
impl Default for Stage {
    fn default() -> Self {
        Self {
            duration_sec: 10,
            rps: 100,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Journey {
    pub  name: String,
    pub  weight: u16,
    pub  steps: Vec<Step>,
}

impl Default for Journey {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            weight: 1,
            steps: Vec::from([Step::default(), Step::default_request()]),
        }
    }
}


#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub  enum StepMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl TryFrom<String> for StepMethod {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "GET" => Ok(StepMethod::GET),
            "POST" => Ok(StepMethod::POST),
            "PUT" => Ok(StepMethod::PUT),
            "PATCH" => Ok(StepMethod::PATCH),
            "DELETE" => Ok(StepMethod::DELETE),
            _ => Err(ValidationError{ path: "/step/".to_string(), code: "invalid_value".to_string(), message: "invalid_value".to_string()}),
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Step {
    Sleep {
        duration_ms: u32,
    },
    Request {
        method: StepMethod,
        path: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        timeout_ms: Option<u32>,
    },
}

impl Default for Step {
    fn default() -> Self {
        Step::Sleep { duration_ms: 0 }
    }
}

impl Step {
    fn default_request() -> Self {
        Request {
            method: StepMethod::GET,
            path: "".to_string(),
            headers: None,
            body: None,
            timeout_ms: None,
        }
    }
}
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub  enum ThresholdOperator {
    Lt,
    Gt,
    Lte,
    Gte,
    Eq
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct Threshold {
    pub  metric: String,
    pub  op: ThresholdOperator,
    pub  value: i32,
    pub  scope: Option<ThresholdScope>,
}

impl Default for Threshold {
    fn default() -> Self {
        Self {
            metric: "http.error_rate".to_string(),
            op: ThresholdOperator::Gt,
            value: 10,
            scope: None,
        }
    }
}


#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct ThresholdScope {
    pub  endpoint: String,
    pub  journey: String,
}