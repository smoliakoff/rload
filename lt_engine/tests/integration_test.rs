use std::path::PathBuf;
use assert_cmd::{cargo, Command};


#[test]
pub fn it_check_validate_command_for_valid_scenario() {
    let scenario_path = fixture_path("valid-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("lt_engine"));

    cmd
        .arg("validate")
        .arg(format!("--scenario={}", scenario_path.display()))
        .assert()
        .code(0);
}

#[test]
pub fn it_check_validate_command_for_invalid_scenario() {
    let scenario_path = fixture_path("invalid-schema-error-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("lt_engine"));

    cmd
        .arg("validate")
        .arg(format!("--scenario={}", scenario_path.display()))
        .assert()
        .code(3);
}
#[test]
pub fn it_check_validate_command_for_broken_json() {
    let scenario_path = fixture_path("broken-json-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("lt_engine"));

    cmd
        .arg("validate")
        .arg(format!("--scenario={}", scenario_path.display()))
        .assert()
        .code(3);
}


fn fixture_path(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("libprotocol")
        .join("tests")
        .join("fixtures")
        .join(name);
    p.canonicalize().unwrap()
}