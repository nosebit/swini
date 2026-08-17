use assert_cmd::Command;
use predicates::prelude::*;

// Since this file is named `daemon.rs`, the top-level module in the test report
// will automatically be `daemon`.
//
// By adding `mod lifecycle` inside it, this test will show up in your terminal
// as `daemon::lifecycle::starts_successfully`.
mod lifecycle {
  use super::*;

  #[test]
  fn starts_successfully() {
    let mut cmd = Command::cargo_bin("swini").unwrap();

    cmd.arg("daemon").arg("start");

    cmd
      .assert()
      .success()
      .stdout(predicate::str::contains("Start the daemon"));
  }
}
