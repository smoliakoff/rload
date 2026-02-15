use std::collections::{BTreeMap, HashMap};
use std::string::ToString;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::schema::Step::Request;
use crate::ValidationError;

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate) struct Scenario {
    pub(crate) version: u16,
    pub(crate) name: String,
    pub(crate) target: Target,
    pub(crate) workload: Workload,
    pub(crate) journeys: Vec<Journey>,
    pub(crate) description: Option<String>,
    pub(crate) tags: Option<Vec<String>>,
    pub(crate) thresholds: Option<Vec<Threshold>>,
    pub(crate) metadata: Option<()>
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
pub(crate)struct Target {
    pub(crate) base_url: String,
    pub(crate) default_headers: Option<BTreeMap<String, String>>,
    pub(crate) insecure_tls: Option<bool>
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
#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate)struct Workload {
    pub(crate) stages: Vec<Stage>,
}
impl Default for Workload {
    fn default() -> Self {
        Self {
            stages: vec![Stage::default()],
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate)struct Stage {
    pub(crate) duration_sec: i32,
    pub(crate) rps: i32,
}
impl Default for Stage {
    fn default() -> Self {
        Self {
            duration_sec: 10,
            rps: 100,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate)struct Journey {
    pub(crate) name: String,
    pub(crate) weight: u16,
    pub(crate) steps: Vec<Step>,
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
pub(crate) enum StepMethod {
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

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Step {
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
pub(crate) enum ThresholdOperator {
    Lt,
    gt,
    Lte,
    Gte,
    Eq
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate)struct Threshold {
    pub(crate) metric: String,
    pub(crate) op: ThresholdOperator,
    pub(crate) value: i32,
    pub(crate) scope: Option<ThresholdScope>,
}

impl Default for Threshold {
    fn default() -> Self {
        Self {
            metric: "http.error_rate".to_string(),
            op: ThresholdOperator::gt,
            value: 10,
            scope: None,
        }
    }
}


#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub(crate)struct ThresholdScope {
    pub(crate) endpoint: String,
    pub(crate) journey: String,
}