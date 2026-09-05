//! End-to-end lifecycle integration tests for the Swini Regent.
//!
//! Tests process supervision, detached daemonization, status inspection,
//! and graceful termination via the `swini regent` CLI commands.

use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn cli_help_shows_regent_subcommand() {
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd.arg("--help");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("regent"));
}

#[test]
fn regent_help_shows_lifecycle_subcommands() {
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd.arg("regent").arg("--help");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("start"))
    .stdout(predicate::str::contains("stop"))
    .stdout(predicate::str::contains("status"));
}

#[test]
fn regent_stop_not_running_reports_informative_error() {
  let temp_home = tempdir().unwrap();
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("nonexistent-croft");

  cmd.assert().failure().stderr(predicate::str::contains(
    "Regent 'nonexistent-croft' is not running",
  ));
}

#[test]
fn regent_status_when_no_regents_reports_empty() {
  let temp_home = tempdir().unwrap();
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("status");

  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("No regents running."));
}

#[test]
fn regent_detached_lifecycle_start_status_stop() {
  let temp_home = tempdir().unwrap();
  let config_path = temp_home.path().join("swini.yml");

  // Choose a random high port
  let port = 10000 + (rand::random::<u16>() % 50000);
  let yaml_content = format!(
    r#"
name: e2e-croft
addr: "127.0.0.1:{port}"
roles:
  - server
  - worker
"#
  );
  std::fs::write(&config_path, yaml_content).unwrap();

  // 1. Start Regent detached
  let mut start_cmd = Command::cargo_bin("swini").unwrap();
  start_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&config_path)
    .arg("-d");

  start_cmd.assert().success();

  // Wait briefly for the daemon to start and write regent.json
  std::thread::sleep(Duration::from_millis(500));

  // 2. Query status
  let mut status_cmd = Command::cargo_bin("swini").unwrap();
  status_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("status");

  status_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("e2e-croft"))
    .stdout(predicate::str::contains("Running"));

  // 3. Stop Regent
  let mut stop_cmd = Command::cargo_bin("swini").unwrap();
  stop_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("e2e-croft");

  stop_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("stopped successfully"));

  // 4. Verify status after stop
  let mut status_after_cmd = Command::cargo_bin("swini").unwrap();
  status_after_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("status");

  status_after_cmd.assert().success().stdout(
    predicate::str::contains("No matching regents found.")
      .or(predicate::str::contains("No regents running.")),
  );
}
