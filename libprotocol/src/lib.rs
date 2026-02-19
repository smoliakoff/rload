use std::io::Write;
mod protocol_error;
mod semantic_validator;
pub mod schema;

use crate::protocol_error::ValidationErrors;
pub use crate::protocol_error::{JsonError, ProtocolError, ValidationError};
pub use crate::schema::Scenario;
use crate::semantic_validator::Validator;
use anyhow::Context;
use schemars::schema_for;
use serde_json::Value;
use std::fs;
use std::io::stderr;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, ProtocolError>;

pub fn parse_scenario(path: impl AsRef<Path>) -> Scenario {
    let path = path.as_ref();
    let json_content = fs::read_to_string(path).unwrap();

    serde_json::from_str(&json_content).unwrap()
}


pub fn validate(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    eprintln!("scenario path: {}", path.display());
    let json_content = fs::read_to_string(path)?;
    let schema = schema_for!(Scenario);
    let schema_json: Value = serde_json::to_value(&schema)
        .map_err(|e| JsonError { line: 0, column: 0, message: e.to_string() })?;

    let scenario_json: Value = serde_json::from_str(&json_content).map_err(|e| JsonError {
        line: e.line(),
        column: e.column(),
        message: e.to_string(),
    })?;

    let validator = jsonschema::validator_for(&schema_json).map_err(|e| {
        ProtocolError::Validation(ValidationErrors {
            items: vec![ValidationError {
                path: "$schema".into(),
                code: "schema_compile_error".into(),
                message: e.to_string(),
            }],
        })
    })?;

    let mut errors: Vec<ValidationError>     = Vec::new();

    for err in validator.iter_errors(&scenario_json) {
        errors.push(ValidationError {
            path: err.instance_path().to_string(), // типа "/workload/stages"
            code: "".to_string(),        // грубо, но ок для MVP
            message: err.to_string(),
        });
    }

    // 2) Business validation errors
    let business = Validator::new()
        .with_rule(crate::semantic_validator::NameRule::new())
        .with_rule(crate::semantic_validator::WebProtocolRule::new())
        .with_rule(crate::semantic_validator::StagesRule::new())
        .with_rule(crate::semantic_validator::DurationRule::new())
        .with_rule(crate::semantic_validator::RpsRule::new())
        .with_rule(crate::semantic_validator::VersionRule::new())
        .with_rule(crate::semantic_validator::JourneysRule::new());
    business.validate(&scenario_json, &mut errors);

    if !errors.is_empty() {
        writeln!(stderr(), "{}", format!("Scenario is invalid ({} errors)", errors.len())).expect("Something went wrong while print stderr");
        return Err(ValidationErrors { items: errors }.into());
    }
    println!("ok");

    Ok(())
}

pub fn export_schema(out_path: impl AsRef<Path>, version: Option<String>) -> anyhow::Result<()> {
    let path = out_path.as_ref();
    let final_path = with_version(path, version.as_deref())?;
    let mut schema = schema_for!(Scenario);
    let v = version.unwrap_or_else(|| "1".to_string());
    schema.insert("$version".to_string(), serde_json::Value::String(v));
    let _ = fs::write(final_path, serde_json::to_string_pretty(&schema).unwrap());
    println!("Schema exported successfully !");

    Ok(())
}

fn with_version(path: &Path, version: Option<&str>) -> anyhow::Result<PathBuf> {
    let Some(version) = version else {
        return Ok(path.to_path_buf());
    };

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid output file name"))?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json");

    let new_name = format!("{stem}-v{version}.{ext}");
    Ok(path.with_file_name(new_name))
}


pub fn generate_scenario(out_path: impl AsRef<Path>, version: &str) -> anyhow::Result<()> {
    let path = out_path.as_ref();
    let mut default_scenario: Scenario = Scenario::default();
    default_scenario = default_scenario.set_version(version.parse().unwrap_or(1u16));
    fs::write(path, serde_json::to_string_pretty(&default_scenario)?)
        .context("Failed to write default scenario to file")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;
    #[test]
    fn it_check_export_schema() -> anyhow::Result<()> {
        let tmp = tempdir()?;
        let output_file = tmp.path().join("result.txt");
        let result_file = tmp.path().join("result-v1.txt");

        export_schema(output_file.to_str().unwrap().to_string(), Some(String::from("1")))?;

        let actual = fs::read_to_string(result_file)?;
        insta::assert_snapshot!(actual);

        Ok(())
    }
}
