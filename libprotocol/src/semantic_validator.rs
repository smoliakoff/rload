use std::iter;
use predicates::Predicate;
use crate::{Scenario, Stage, ValidationError};


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


pub(crate) struct WebProtocolRule {
    message: String
}

impl WebProtocolRule {
    pub(crate) fn new() -> Self {
        WebProtocolRule {
            message: "url must start with http or https".to_string()
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