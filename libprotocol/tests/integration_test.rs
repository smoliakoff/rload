use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use libprotocol::{validate};

#[test]
fn it_check_generate_scenario() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let output_file = tmp.path().join("result.txt");

    let _ = libprotocol::generate_scenario(output_file.to_str().unwrap(), "1");

    insta::assert_snapshot!(std::fs::read_to_string(&output_file)?);

    Ok(())
}
#[test]
fn it_check_export_schema() -> anyhow::Result<()> {
    let tmp = tempdir()?;
    let version = 1;
    let output_file = tmp.path().join("schema.json");

    libprotocol::export_schema(&output_file, Some(version.to_string()))?;
    let result_path = tmp.path().join(format!("schema-v{:?}.json", version));

    insta::assert_debug_snapshot!(fs::read_to_string(&result_path)?);

    Ok(())
}

#[test]
fn it_check_validate_with_valid_scenario() -> anyhow::Result<()>
{
    let scenario_file = fixture_path("valid-scenario.json");
    let _ = fs::read(scenario_file.to_str().unwrap());
    validate(scenario_file)?;
    Ok(())
}
#[test]
fn it_check_validate_with_invalid_scenario_broken_json()
{
    let scenario_file = fixture_path("broken-json-scenario.json");
    let err = validate(scenario_file).unwrap_err();
    match err {
        libprotocol::ProtocolError::Json(e) => {
            assert_eq!(e.line, 9);
            assert_eq!(e.column, 0);
            assert_eq!(e.message, "EOF while parsing an object at line 9 column 0");
        }
        other => panic!("Expected Json error, got: {other:?}"),
    }
}

#[test]
fn it_check_validate_with_invalid_scenario_validation_schema_error()
{
    let scenario_file = fixture_path("invalid-schema-error-scenario.json");
    let err = validate(scenario_file).unwrap_err();
    match err {
        libprotocol::ProtocolError::Validation(e) => {
            insta::assert_debug_snapshot!(e);
        }
        other => panic!("Expected Json error, got: {other:?}"),
    }
}
#[test]
fn it_check_validate_with_invalid_scenario_validation_semantic_rules()
{
    let scenario_file = fixture_path("invalid-semantic-rules-scenario.json");
    let err = validate(scenario_file).unwrap_err();
    match err {
        libprotocol::ProtocolError::Validation(e) => {
            insta::assert_debug_snapshot!(e);
        }
        other => panic!("Expected Json error, got: {other:?}"),
    }
}
#[test]
fn it_check_validate_with_invalid_scenario_validation_semantic_rules_empty_stages()
{
    let scenario_file = fixture_path("invalid-semantic-rules-scenario-empty-stages.json");
    let err = validate(scenario_file).unwrap_err();
    match err {
        libprotocol::ProtocolError::Validation(e) => {
            insta::assert_debug_snapshot!(e);
        }
        other => panic!("Expected Json error, got: {other:?}"),
    }
}#[test]
fn it_check_validate_with_invalid_extended_scenario_validation_semantic_rules_wrong_steps()
{
    let scenario_file = fixture_path("invalid-extended-scenario.json");
    let err = validate(scenario_file).unwrap_err();
    match err {
        libprotocol::ProtocolError::Validation(e) => {
            insta::assert_debug_snapshot!(e);
        }
        other => panic!("Expected Json error, got: {other:?}"),
    }
}
#[test]
fn it_check_validate_with_valid_extended_scenario()
{
    let scenario_file = fixture_path("valid-extended-scenario.json");
    validate(scenario_file).unwrap();
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}