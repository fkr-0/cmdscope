use assert_cmd::Command;
use predicates::prelude::*;
use rusqlite::{Connection, params};
use tempfile::tempdir;

#[test]
fn version_flag_reports_the_package_version() {
    Command::cargo_bin("cmdscope")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("cmdscope 0.2.2"));
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

#[test]
fn print_first_preserves_shell_metacharacters_as_plain_stdout_text() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("history.db");
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command text not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );",
        )
        .unwrap();
    let command = "printf '%s' \"$HOME\"; echo '$PATH' | sed 's/[&]/_/g' && echo Ω\nnext line";
    connection
        .execute(
            "insert into history values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params!["shell", 20_i64, 0_i64, 0_i64, command, "/repo", "s", "h"],
        )
        .unwrap();

    Command::cargo_bin("cmdscope")
        .unwrap()
        .env_remove("CMDSCOPE_CONFIG")
        .env("XDG_CONFIG_HOME", directory.path().join("config"))
        .args(["--db", database.to_str().unwrap(), "--print-first"])
        .assert()
        .success()
        .stdout(format!("{command}\n"));
}

#[test]
fn null_terminated_output_preserves_trailing_newlines() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("history.db");
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command text not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );",
        )
        .unwrap();
    let command = "printf one\n\n";
    connection
        .execute(
            "insert into history values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params!["nul", 20_i64, 0_i64, 0_i64, command, "/repo", "s", "h"],
        )
        .unwrap();

    Command::cargo_bin("cmdscope")
        .unwrap()
        .env_remove("CMDSCOPE_CONFIG")
        .env("XDG_CONFIG_HOME", directory.path().join("config"))
        .args([
            "--db",
            database.to_str().unwrap(),
            "--print-first",
            "--null",
        ])
        .assert()
        .success()
        .stdout(format!("{command}\0"));
}

#[test]
fn print_first_supports_old_schema_and_exact_multiline_output() {
    let directory = tempdir().unwrap();
    let database = directory.path().join("history.db");
    let connection = Connection::open(&database).unwrap();
    connection
        .execute_batch(
            "create table history (
                id text primary key,
                timestamp integer not null,
                duration integer not null,
                exit integer not null,
                command text not null,
                cwd text not null,
                session text not null,
                hostname text not null
            );",
        )
        .unwrap();
    connection
        .execute(
            "insert into history values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                "old",
                20_i64,
                0_i64,
                0_i64,
                "echo one\necho two",
                "/repo",
                "s",
                "h"
            ],
        )
        .unwrap();

    Command::cargo_bin("cmdscope")
        .unwrap()
        .env_remove("CMDSCOPE_CONFIG")
        .env("XDG_CONFIG_HOME", directory.path().join("config"))
        .args(["--db", database.to_str().unwrap(), "--print-first"])
        .assert()
        .success()
        .stdout("echo one\necho two\n");
}
