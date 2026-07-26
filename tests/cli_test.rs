use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn version_flag_reports_the_package_version() {
    Command::cargo_bin("cmdscope")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("cmdscope 0.2.1"));
}

#[test]
fn explicit_missing_config_is_an_error() {
    let directory = tempdir().unwrap();

    Command::cargo_bin("cmdscope")
        .unwrap()
        .args([
            "--config",
            directory.path().join("missing.toml").to_str().unwrap(),
            "--db",
            directory.path().join("missing.db").to_str().unwrap(),
            "--print-first",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("failed to read config"));
}
