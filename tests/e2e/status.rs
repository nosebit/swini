//! End-to-end integration tests for `swini status` and `swini croft status`.
//!
//! Tests cluster-wide status queries, single-node resource accounting display,
//! and live telemetry sampling against a running Regent daemon.

use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn cli_help_shows_status_and_croft_subcommands() {
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd.arg("--help");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("status"))
    .stdout(predicate::str::contains("croft"));
}

#[test]
fn croft_help_shows_status_subcommand() {
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd.arg("croft").arg("--help");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("status"));
}

#[test]
fn cluster_status_and_node_detail_e2e() {
  let temp_home = tempdir().unwrap();
  let config_path = temp_home.path().join("swini.yml");

  let port = 10000 + (rand::random::<u16>() % 50000);
  let yaml_content = format!(
    r#"
name: cluster-status-croft
addr: "127.0.0.1:{port}"
roles:
  - server
  - worker
tags:
  - primary-zone
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

  // Give the daemon time to boot, probe resources, and initialize Raft
  std::thread::sleep(Duration::from_millis(800));

  // 2. Run cluster-wide status
  let mut status_cmd = Command::cargo_bin("swini").unwrap();
  status_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{port}"))
    .arg("status");

  status_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("ID"))
    .stdout(predicate::str::contains("NAME"))
    .stdout(predicate::str::contains("CPU (RES/YARD/TOT)"))
    .stdout(predicate::str::contains("MEM (RES/YARD/TOT)"))
    .stdout(predicate::str::contains("cluster-status-croft"))
    .stdout(predicate::str::contains("Active"));

  // 3. Run single node status without live telemetry
  let mut node_status_cmd = Command::cargo_bin("swini").unwrap();
  node_status_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{port}"))
    .arg("croft")
    .arg("status")
    .arg("cluster-status-croft");

  node_status_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("Croft: cluster-status-croft"))
    .stdout(predicate::str::contains("RESOURCES (BARN)"))
    .stdout(predicate::str::contains("CPU Total:"))
    .stdout(predicate::str::contains("CPU Yardable:"))
    .stdout(predicate::str::contains("Memory Total:"))
    .stdout(predicate::str::contains("Memory Yardable:"))
    .stdout(predicate::str::contains("LIVE TELEMETRY").not());

  // 4. Run single node status WITH live telemetry
  let mut live_status_cmd = Command::cargo_bin("swini").unwrap();
  live_status_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{port}"))
    .arg("croft")
    .arg("status")
    .arg("cluster-status-croft")
    .arg("--live");

  live_status_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("Croft: cluster-status-croft"))
    .stdout(predicate::str::contains("RESOURCES (BARN)"))
    .stdout(predicate::str::contains("LIVE TELEMETRY"))
    .stdout(predicate::str::contains("Live CPU Usage:"))
    .stdout(predicate::str::contains("Live Memory Used:"));

  // 5. Query status of nonexistent node
  let mut not_found_cmd = Command::cargo_bin("swini").unwrap();
  not_found_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{port}"))
    .arg("croft")
    .arg("status")
    .arg("nonexistent-node");

  not_found_cmd
    .assert()
    .failure()
    .stderr(predicate::str::contains(
      "Croft 'nonexistent-node' not found",
    ));

  // 6. Clean up: Stop Regent
  let mut stop_cmd = Command::cargo_bin("swini").unwrap();
  stop_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("cluster-status-croft");

  stop_cmd.assert().success();
}
