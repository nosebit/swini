//! End-to-end integration tests for workload execution (`swini pig`).
//!
//! Tests declarative YAML workload deployment, Drover scheduling, Supervisor
//! process execution, heterogeneous capacity-based placement, Primary failover,
//! and graceful termination via `swini pig run` and `swini pig stop`.

use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;
use tempfile::tempdir;

#[test]
fn cli_help_shows_pig_subcommand() {
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd.arg("--help");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("pig"));
}

#[test]
fn pig_help_shows_run_and_stop_subcommands() {
  let mut cmd = Command::cargo_bin("swini").unwrap();
  cmd.arg("pig").arg("--help");
  cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("run"))
    .stdout(predicate::str::contains("stop"));
}

#[test]
fn pig_run_and_stop_lifecycle_e2e() {
  let temp_home = tempdir().unwrap();
  let croft_config = temp_home.path().join("swini.yml");

  let port = 10000 + (rand::random::<u16>() % 40000);
  let croft_yaml = format!(
    r#"
name: pig-lifecycle-croft
addr: "127.0.0.1:{port}"
roles:
  - server
  - worker
resources:
  cpu: "4 GHz"
  mem: "4 GB"
"#
  );
  std::fs::write(&croft_config, croft_yaml).unwrap();

  // 1. Start Regent in detached mode
  let mut start_cmd = Command::cargo_bin("swini").unwrap();
  start_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&croft_config)
    .arg("-d");

  start_cmd.assert().success();
  std::thread::sleep(Duration::from_millis(700));

  // 2. Create pigs.yml with string units and case insensitivity ("8mb", "100
  //    mhz")
  let pig_config_path = temp_home.path().join("pigs.yml");
  let pig_yaml = r#"
space: main
pigs:
  - name: test-app
    size: 1
    piglet:
      placement:
        yard:
          cpu: "100 mhz"
          mem: "8mb"
      tasks:
        - name: sleeper
          exec:
            command: ["sleep", "30"]
"#;
  std::fs::write(&pig_config_path, pig_yaml).unwrap();

  // 3. Deploy workload via `swini pig run`
  let mut run_cmd = Command::cargo_bin("swini").unwrap();
  run_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{port}"))
    .arg("pig")
    .arg("run")
    .arg("-c")
    .arg(&pig_config_path);

  run_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("ok"));

  std::thread::sleep(Duration::from_millis(800));

  // 4. Stop workload via `swini pig stop`
  let mut stop_pig_cmd = Command::cargo_bin("swini").unwrap();
  stop_pig_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{port}"))
    .arg("pig")
    .arg("stop")
    .arg("-c")
    .arg(&pig_config_path)
    .arg("test-app");

  stop_pig_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("ok"));

  // 5. Clean up Regent
  let mut stop_regent_cmd = Command::cargo_bin("swini").unwrap();
  stop_regent_cmd
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("pig-lifecycle-croft");

  stop_regent_cmd.assert().success();
}

#[test]
fn heterogeneous_multi_node_placement_e2e() {
  let temp_home = tempdir().unwrap();
  let base_port = 15000 + (rand::random::<u16>() % 35000);
  let p1 = base_port;
  let p2 = base_port + 1;

  // Node 1: Seed server + small capacity worker (using numbers defaulting to
  // MHz/MB)
  let node1_config = temp_home.path().join("node1.yml");
  let node1_yaml = format!(
    r#"
name: node-small
addr: "127.0.0.1:{p1}"
roles:
  - server
  - worker
tags:
  - zone-a
resources:
  cpu: 1000.0
  mem: 1000.0
"#
  );
  std::fs::write(&node1_config, node1_yaml).unwrap();

  // Node 2: Dedicated large capacity GPU worker (using unit strings)
  let node2_config = temp_home.path().join("node2.yml");
  let node2_yaml = format!(
    r#"
name: node-large-gpu
addr: "127.0.0.1:{p2}"
roles:
  - worker
tags:
  - zone-b
  - gpu
resources:
  cpu: "8 GHz"
  mem: "8 GB"
join_addresses:
  - "127.0.0.1:{p1}"
"#
  );
  std::fs::write(&node2_config, node2_yaml).unwrap();

  // Start Node 1
  let mut start1 = Command::cargo_bin("swini").unwrap();
  start1
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&node1_config)
    .arg("-d");
  start1.assert().success();
  std::thread::sleep(Duration::from_millis(600));

  // Start Node 2
  let mut start2 = Command::cargo_bin("swini").unwrap();
  start2
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&node2_config)
    .arg("-d");
  start2.assert().success();
  std::thread::sleep(Duration::from_millis(800));

  // Create workload manifest targeting specific GPU resources
  let pig_config_path = temp_home.path().join("heterogeneous_pigs.yml");
  let pig_yaml = r#"
space: main
pigs:
  - name: gpu-workload
    size: 1
    piglet:
      placement:
        croft:
          tags:
            - gpu
        yard:
          cpu: "2 GHz"
          mem: "4 gb"
      tasks:
        - name: trainer
          exec:
            command: ["sleep", "30"]
"#;
  std::fs::write(&pig_config_path, pig_yaml).unwrap();

  // Deploy workload via Node 1
  let mut run_cmd = Command::cargo_bin("swini").unwrap();
  run_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{p1}"))
    .arg("pig")
    .arg("run")
    .arg("-c")
    .arg(&pig_config_path);

  run_cmd
    .assert()
    .success()
    .stdout(predicate::str::contains("ok"));

  std::thread::sleep(Duration::from_millis(800));

  // Stop workload
  let mut stop_pig_cmd = Command::cargo_bin("swini").unwrap();
  stop_pig_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{p1}"))
    .arg("pig")
    .arg("stop")
    .arg("-c")
    .arg(&pig_config_path)
    .arg("gpu-workload");

  stop_pig_cmd.assert().success();

  // Cleanup
  let mut stop_cmd1 = Command::cargo_bin("swini").unwrap();
  stop_cmd1
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("node-small");
  stop_cmd1.assert().success();

  let mut stop_cmd2 = Command::cargo_bin("swini").unwrap();
  stop_cmd2
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("node-large-gpu");
  stop_cmd2.assert().success();
}

#[test]
fn primary_croft_failover_e2e() {
  let temp_home = tempdir().unwrap();
  let base_port = 20000 + (rand::random::<u16>() % 30000);
  let p1 = base_port;
  let p2 = base_port + 1;
  let p3 = base_port + 2;

  // 3 Server nodes for Raft Quorum
  let s1_config = temp_home.path().join("server1.yml");
  let s1_yaml = format!(
    r#"
name: quorum-s1
addr: "127.0.0.1:{p1}"
roles:
  - server
  - worker
resources:
  cpu: "4 GHz"
  mem: "4 GB"
"#
  );
  std::fs::write(&s1_config, s1_yaml).unwrap();

  let s2_config = temp_home.path().join("server2.yml");
  let s2_yaml = format!(
    r#"
name: quorum-s2
addr: "127.0.0.1:{p2}"
roles:
  - server
  - worker
resources:
  cpu: "4 GHz"
  mem: "4 GB"
join_addresses:
  - "127.0.0.1:{p1}"
"#
  );
  std::fs::write(&s2_config, s2_yaml).unwrap();

  let s3_config = temp_home.path().join("server3.yml");
  let s3_yaml = format!(
    r#"
name: quorum-s3
addr: "127.0.0.1:{p3}"
roles:
  - server
  - worker
resources:
  cpu: "4 GHz"
  mem: "4 GB"
join_addresses:
  - "127.0.0.1:{p1}"
  - "127.0.0.1:{p2}"
"#
  );
  std::fs::write(&s3_config, s3_yaml).unwrap();

  // Start s1 (initial leader)
  let mut start1 = Command::cargo_bin("swini").unwrap();
  start1
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&s1_config)
    .arg("-d");
  start1.assert().success();
  std::thread::sleep(Duration::from_millis(800));

  // Start s2 and s3
  let mut start2 = Command::cargo_bin("swini").unwrap();
  start2
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&s2_config)
    .arg("-d");
  start2.assert().success();
  std::thread::sleep(Duration::from_millis(800));

  let mut start3 = Command::cargo_bin("swini").unwrap();
  start3
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("start")
    .arg("-c")
    .arg(&s3_config)
    .arg("-d");
  start3.assert().success();
  std::thread::sleep(Duration::from_millis(1200));

  // Deploy workload to the cluster via s1
  let pig_config_path = temp_home.path().join("failover_pigs.yml");
  let pig_yaml = r#"
space: main
pigs:
  - name: failover-app
    size: 2
    piglet:
      placement:
        yard:
          cpu: "100 MHz"
          mem: "100 MB"
      tasks:
        - name: service
          exec:
            command: ["sleep", "30"]
"#;
  std::fs::write(&pig_config_path, pig_yaml).unwrap();

  let mut run_cmd = Command::cargo_bin("swini").unwrap();
  run_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{p1}"))
    .arg("pig")
    .arg("run")
    .arg("-c")
    .arg(&pig_config_path);
  run_cmd.assert().success();

  std::thread::sleep(Duration::from_millis(800));

  // Kill s1 (the primary)
  let mut stop_s1 = Command::cargo_bin("swini").unwrap();
  stop_s1
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("quorum-s1");
  stop_s1.assert().success();

  // Wait for new leader election and Drover promotion on s2 or s3
  std::thread::sleep(Duration::from_millis(2500));

  // Stop workload via surviving node s2
  let mut stop_pig_cmd = Command::cargo_bin("swini").unwrap();
  stop_pig_cmd
    .env("HOME", temp_home.path())
    .env("SWINI_ADDR", format!("127.0.0.1:{p2}"))
    .arg("pig")
    .arg("stop")
    .arg("-c")
    .arg(&pig_config_path)
    .arg("failover-app");

  stop_pig_cmd.assert().success();

  // Cleanup surviving nodes
  let mut stop_s2 = Command::cargo_bin("swini").unwrap();
  stop_s2
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("quorum-s2");
  stop_s2.assert().success();

  let mut stop_s3 = Command::cargo_bin("swini").unwrap();
  stop_s3
    .env("HOME", temp_home.path())
    .arg("regent")
    .arg("stop")
    .arg("quorum-s3");
  stop_s3.assert().success();
}
