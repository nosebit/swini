use assert_cmd::Command;

mod status {
  use super::*;

  #[test]
  fn status_fails_gracefully_when_no_server_running() {
    let mut cmd = Command::cargo_bin("swini").unwrap();
    cmd.env("SWINI_API_URL", "http://127.0.0.1:59990");
    cmd.arg("status");
    cmd.assert().failure();
  }
}
