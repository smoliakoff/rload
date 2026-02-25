use assert_cmd::{cargo, Command};
use test_support::fixture_path;

#[test]
pub fn it_check_validate_command_for_valid_scenario() {
    let scenario_path = fixture_path("crates/libprotocol/tests/fixtures/valid-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("rload"));

    cmd
        .arg("validate")
        .arg(format!("--scenario={}", scenario_path.display()))
        .assert()
        .code(0);
}

#[test]
pub fn it_check_dry_run_command_for_valid_scenario() {
    let scenario_path = fixture_path("crates/libprotocol/tests/fixtures/valid-extended-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("rload"));

    cmd
        .arg("dry-run")
        .arg(format!("--scenario={}", scenario_path.display()))
        .arg("--iterations=100")
        .assert()
        .code(0);
    insta::assert_debug_snapshot!(cmd.output())
}

#[test]
pub fn it_check_validate_command_for_invalid_scenario() {
    let scenario_path = fixture_path("crates/libprotocol/tests/fixtures/invalid-schema-error-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("rload"));

    cmd
        .arg("validate")
        .arg(format!("--scenario={}", scenario_path.display()))
        .assert()
        .code(3);
}
#[test]
pub fn it_check_validate_command_for_broken_json() {
    let scenario_path = fixture_path("crates/libprotocol/tests/fixtures/broken-json-scenario.json");

    let mut cmd = Command::new(cargo::cargo_bin!("rload"));

    cmd
        .arg("validate")
        .arg(format!("--scenario={}", scenario_path.display()))
        .assert()
        .code(3);
}