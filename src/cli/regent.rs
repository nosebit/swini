//! CLI subcommand handlers and argument structures for Swini Regent.
//!
//! Exposes [`Command`] for scoped Regent operations (`start`, `stop`, `status`)
//! and delegates execution via [`run`].

use clap::Subcommand;
use std::error::Error;
use std::path::PathBuf;

/// Scoped subcommands for managing the Regent runtime process.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Starts the Swini Regent in foreground (default) or background
  /// (--detached)
  Start {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Run detached in the background
    #[arg(short, long)]
    detached: bool,
  },
  /// Stops a running Swini Regent
  Stop {
    /// Regent name (defaults to "main")
    name: Option<String>,
  },
  /// Inspects Regent status or Ranch crofts
  Status {
    /// Specific Regent name to query
    name: Option<String>,
  },
}

/// Forks the process into the background via [`daemonize::Daemonize`] before
/// async runtime threads are initialized.
pub fn daemonize() -> Result<(), Box<dyn Error>> {
  let dev_null = std::fs::File::create("/dev/null")?;
  let daemonize = daemonize::Daemonize::new()
    .working_directory(
      std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/")),
    )
    .stdout(dev_null.try_clone()?)
    .stderr(dev_null);

  daemonize.start()?;
  Ok(())
}

/// Executes Regent subcommands by delegating to [`crate::regent`].
pub async fn run(cmd: Command) -> Result<(), Box<dyn Error>> {
  match cmd {
    Command::Start { config, detached } => {
      crate::regent::start(config, detached).await
    }
    Command::Stop { name } => crate::regent::stop(name).await,
    Command::Status { name } => crate::regent::status(name).await,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use clap::Parser;

  #[derive(Parser, Debug)]
  struct TestCli {
    #[command(subcommand)]
    command: Command,
  }

  #[test]
  fn cli_parse_start_command() {
    let cli = TestCli::try_parse_from(["swini", "start"]).unwrap();
    assert_eq!(
      cli.command,
      Command::Start {
        config: None,
        detached: false,
      }
    );

    let cli =
      TestCli::try_parse_from(["swini", "start", "-c", "custom.yml", "-d"])
        .unwrap();
    assert_eq!(
      cli.command,
      Command::Start {
        config: Some(PathBuf::from("custom.yml")),
        detached: true,
      }
    );
  }

  #[test]
  fn cli_parse_stop_and_status() {
    let stop_cli =
      TestCli::try_parse_from(["swini", "stop", "worker-1"]).unwrap();
    assert_eq!(
      stop_cli.command,
      Command::Stop {
        name: Some("worker-1".to_string()),
      }
    );

    let status_cli = TestCli::try_parse_from(["swini", "status"]).unwrap();
    assert_eq!(status_cli.command, Command::Status { name: None });
  }
}
