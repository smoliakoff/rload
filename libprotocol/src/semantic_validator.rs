use predicates::Predicate;
use crate::schema::{Journey, Scenario, Step};
use crate::schema::Step::{Request, Sleep};
use crate::ValidationError;

enum ScenarioVersion {
    V1 = 1,
}
impl TryFrom<u16> for ScenarioVersion {
    type Error = ValidationError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(ScenarioVersion::V1),
            _ => Err(ValidationError{ path: "/version".to_string(), code: "unsupported_version".to_string(), message: "Unsupported version".to_string()}),
        }
    }
}

pub struct Validator {
    rules: Vec<Box<dyn Rule>>,
}

impl Validator {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }
    pub fn with_rule(mut self, rule: impl Rule + 'static) -> Self {
        self.rules.push(Box::new(rule));
        self
    }

    pub fn validate(&self, v: &serde_json::Value, errors: &mut Vec<ValidationError>) {
        let scenario: Scenario = match serde_json::from_value(v.clone()) {
            Ok(v) => v,
            Err(e) => {
                errors.push(ValidationError {
                    path: "".to_string(),
                    code: "json_parse_error".to_string(),
                    message: format!("Failed to parse JSON: {}", e),
                });
                return;
            }
        };
        for r in &self.rules {
            r.validate(&scenario, errors);
        }
    }
}

pub trait Rule: Send + Sync {
    fn validate(&self, v: &Scenario, errors: &mut Vec<ValidationError>);
}

pub(crate) struct NameRule {
    message: String,
}
impl NameRule {
    pub(crate) fn new() -> Self {
        NameRule { message: "name required and must be great then 0".to_string() }
    }
}


pub(crate) struct StagesRule {
    message: String,
}

impl StagesRule {
    pub(crate) fn new() -> Self {
        StagesRule { message: "Stages must be non empty array".to_string() }
    }
}

impl Rule for StagesRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        if scenario.workload.stages.len() == 0 {
            errors.push(ValidationError {
                path: "/workload/stages".to_string(),
                code: "".to_string(),
                message: self.message.clone(),
            })
        }
    }
}

pub(crate) struct DurationRule {
    message: String,
}

impl DurationRule {
    pub(crate) fn new() -> Self {
        DurationRule { message: "Duration must be >= 10 and < 86400 sec ".to_string() }
    }
}

impl Rule for DurationRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        for (i, stage) in scenario.clone().workload.stages.iter().enumerate() {
            if stage.duration_sec < 10 || stage.duration_sec > 86400 {
                errors.push(ValidationError {
                    path: std::format!("/workload/stages/{}/duration_sec", i),
                    code: "".to_string(),
                    message: self.message.clone(),
                })
            }
        }
    }
}

pub(crate) struct VersionRule {
    message: String,
}

impl VersionRule {
    pub(crate) fn new() -> Self {
        VersionRule { message: "version must be integer > 0 and <= 50_000 ".to_string() }
    }
}

impl Rule for VersionRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        let version: u16 = scenario.version as u16;
        let mut push_error = || {
            errors.push(ValidationError {
                path: "/version".to_string(),
                code: "".to_string(),
                message: self.message.clone(),
            })
        };

        if ScenarioVersion::try_from(scenario.version).is_err() {
            errors.push(ValidationError {
                path: "/version".to_string(),
                code: "unsupported_version".to_string(),
                message: format!("Unsupported version: {}. Supported: [1]", scenario.version),
            });
        }
    }
}


pub(crate) struct RpsRule {
    message: String,
}

impl RpsRule {
    pub(crate) fn new() -> Self {
        RpsRule { message: "Rps must be > 1 and < 100 ".to_string() }
    }
}

impl Rule for RpsRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        for (i, stage) in scenario.clone().workload.stages.iter().enumerate() {
            if stage.rps == 0 || stage.rps > 10000 {
                errors.push(ValidationError {
                    path: std::format!("/workload/stages/{}/rps", i),
                    code: "".to_string(),
                    message: self.message.clone(),
                })
            }
        }
    }
}
pub(crate) struct JourneysRule {
    message: String,
}

impl JourneysRule {
    pub(crate) fn new() -> Self {
        JourneysRule { message: "Journeys must be array of Journey. required ".to_string() }
    }
}

impl Rule for JourneysRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        let journeys: &Vec<Journey> = &scenario.journeys;
        if journeys.len() == 0 {
            errors.push(ValidationError {
                path: "/journeys".to_string(),
                code: "missing_field".to_string(),
                message: self.message.clone(),
            });
            return;
        }
        for (i, journey) in journeys.iter().enumerate() {
            if journey.name.is_empty() {
                errors.push(ValidationError {
                    path: std::format!("/journeys/{}/name", i),
                    code: "invalid_value".to_string(),
                    message: "name must be filled".to_string(),
                })
            }
            if journey.weight < 1 || journey.weight > 10000 {
                errors.push(ValidationError {
                    path: std::format!("/journeys/{}/weight", i),
                    code: "invalid_value".to_string(),
                    message: "weight must be >= 1 and < 10000".to_string(),
                })
            }
            for (stepIndex, step) in journey.steps.iter().enumerate() {
                JourneyStepRule::validate(&JourneyStepRule::new(), &step, errors, i, stepIndex);
            }
        }
    }
}


pub(crate) struct JourneyStepRule {
    message: String
}

impl JourneyStepRule {
    pub(crate) fn new() -> Self {
        JourneyStepRule {
            message: "journey step must be valid".to_string()
        }
    }
}
impl JourneyStepRule {
    fn validate(&self, step: &Step, errors: &mut Vec<ValidationError>, journey_index: usize, step_index: usize) {
        match step {
            Sleep {duration_ms, .. } => {
                if duration_ms == &0 || duration_ms > &10000 {
                    errors.push(ValidationError {
                        path: std::format!("/journeys/{}/steps/{}/duration_ms", journey_index, step_index),
                        code: "invalid_value".to_string(),
                        message: "duration_ms must be between 1 and 10000".to_string(),
                    })
                }
            }
            Request {
                path,
                body,
                headers,
                method: _method,
                timeout_ms} => {
                if let Some(timeout_ms) = timeout_ms {
                    if timeout_ms == &0 || timeout_ms > &100000 {
                        errors.push(ValidationError {
                            path: std::format!("/journeys/{}/steps/{}/timeout_ms", journey_index, step_index),
                            code: "invalid_value".to_string(),
                            message: "timeout_ms must be between 1 and 100000".to_string(),
                        })
                    }
                }
                if !path.as_str().starts_with("/") {
                    errors.push(ValidationError {
                        path: std::format!("/journeys/{}/steps/{}/path", journey_index, step_index),
                        code: "invalid_value".to_string(),
                        message: "path required. path must be relative, starts with '/'".to_string(),
                    })
                }
                if let Some(headers) = headers {
                    if headers.len() > 100 {
                        errors.push(ValidationError {
                            path: std::format!("/journeys/{}/steps/{}/headers", journey_index, step_index),
                            code: "invalid_value".to_string(),
                            message: "headers must be less than 100 items".to_string(),
                        })
                    }
                }
                if let Some(body) = body {
                    if body.len() > 10000 {
                        errors.push(ValidationError {
                            path: std::format!("/journeys/{}/steps/{}/body", journey_index, step_index),
                            code: "invalid_value".to_string(),
                            message: "body must be less than 10000 characters".to_string(),
                        })
                    }
                }
            }
        }
    }
}
pub(crate) struct WebProtocolRule {
    message: String
}

impl WebProtocolRule {
    pub(crate) fn new() -> Self {
        WebProtocolRule {
            message: "url must starts with http or https".to_string()
        }
    }
}

impl Rule for WebProtocolRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        let predicates_http = predicates::str::starts_with("http");
        let predicates_https = predicates::str::starts_with("https");
        if !(
            predicates_https.eval(scenario.target.base_url.as_str()) ||
            predicates_http.eval(scenario.target.base_url.as_str())
        )  {
            errors.push(ValidationError{
                path: "/target/base_url".to_string(),
                code: "".to_string(),
                message: self.message.clone(),
            });
        }
    }
}

impl Rule for NameRule {
    fn validate(&self, scenario: &Scenario, errors: &mut Vec<ValidationError>) {
        if scenario.name.len() == 0 {
            errors.push(ValidationError{
                path: "".to_string(),
                code: "".to_string(),
                message: self.message.clone(),
            });
        }
    }
}