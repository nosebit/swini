//! Command-line interface definitions and dispatch for Swini.
//!
//! Exposes top-level command routing via [`Cli`] and [`run`], along with
//! endpoint address resolution in [`resolve_api_url`].
//!
//! Submodules:
//! - [`croft`]: Scoped subcommands for inspecting and managing Crofts.
//! - [`regent`]: Scoped subcommands for managing local Regent lifecycle.
//! - [`status`]: Cluster-wide status query and summary table presentation.

pub mod croft;
pub mod pig;
mod regent;
pub mod status;

pub(crate) use croft::Command as CroftCommand;
pub(crate) use pig::Command as PigCommand;
pub(crate) use regent::Command as RegentCommand;

use crate::croft::config;
use clap::{Parser, Subcommand};
use std::error::Error;
use std::path::PathBuf;

/// Command-line argument structure for Swini.
#[derive(Parser, Debug, PartialEq, Eq)]
#[command(author, version, about = "Swini Workload Orchestrator", long_about = None)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Command,
}

/// Top-level subcommand dispatch.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Displays a formatted summary table of all Crofts across the Ranch
  Status,
  /// Scoped subcommands for inspecting and managing Crofts
  Croft {
    #[command(subcommand)]
    command: CroftCommand,
  },
  /// Subcommands for deploying, inspecting, and stopping Pig workloads
  Pig {
    #[command(subcommand)]
    command: PigCommand,
  },
  /// Manages the local Swini Regent process
  Regent {
    #[command(subcommand)]
    command: RegentCommand,
  },
}

/// Resolves the API endpoint URL for a Croft:
/// 1. From `SWINI_ADDR` environment variable if set (including `local://{name}`
///    URIs).
/// 2. Falling back to default `http://127.0.0.1:7440`.
pub fn resolve_api_url() -> Result<String, Box<dyn Error>> {
  let find_state = |name: &str| -> Option<crate::regent::RegentState> {
    let candidates = [
      config::default_data_dir(name),
      PathBuf::from("sandbox/.data").join(name),
      PathBuf::from(".data").join(name),
    ];
    for dir in &candidates {
      if let Ok(state) = crate::regent::RegentState::load(dir) {
        return Some(state);
      }
    }
    None
  };

  if let Ok(url) = std::env::var("SWINI_ADDR") {
    let trimmed = url.trim();
    if let Some(name) = trimmed.strip_prefix("local://") {
      if let Some(state) = find_state(name) {
        return Ok(format!("http://{}", state.config.addr));
      }
      return Err(
        format!("Local croft '{}' is not running or has no state", name).into(),
      );
    }
    if !trimmed.is_empty() {
      let full_url =
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
          trimmed.to_string()
        } else {
          format!("http://{}", trimmed)
        };
      return Ok(full_url);
    }
  }

  Ok(format!("http://{}", config::DEFAULT_ADDR))
}

/// Builds a configured tonic [`tonic::transport::Endpoint`] by resolving the
/// target API address from `SWINI_ADDR` (or default) and applying configured
/// connection and request timeouts from [`Config::load`].
pub fn resolve_endpoint() -> Result<tonic::transport::Endpoint, Box<dyn Error>>
{
  let api_url = resolve_api_url()?;
  let config = config::Config::load(None).unwrap_or_default();

  let endpoint = tonic::transport::Endpoint::from_shared(api_url)?
    .connect_timeout(config.timeouts.connect)
    .timeout(config.timeouts.request);

  Ok(endpoint)
}

/// Dispatches pre-parsed CLI commands inside an active async runtime.
async fn dispatch(cli: Cli) -> Result<(), Box<dyn Error>> {
  match cli.command {
    Command::Regent { command } => regent::run(command).await,
    remote_cmd => {
      let endpoint = resolve_endpoint()?;
      match remote_cmd {
        Command::Status => status::run(endpoint).await,
        Command::Croft { command } => croft::run(command, endpoint).await,
        Command::Pig { command } => pig::run(command, endpoint).await,
        Command::Regent { .. } => unreachable!(),
      }
    }
  }
}

/// Executes pre-parsed CLI commands, daemonizing first if `--detached` is
/// requested before spinning up the multi-threaded Tokio runtime.
pub fn run_with_cli(cli: Cli) -> Result<(), Box<dyn Error>> {
  if let Command::Regent {
    command: RegentCommand::Start { detached: true, .. },
  } = &cli.command
  {
    if let Err(e) = regent::daemonize() {
      eprintln!("Failed to start regent in background: {}", e);
      std::process::exit(1);
    }
  }

  tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?
    .block_on(dispatch(cli))
}

/// Main entry point parsing CLI arguments and executing commands.
pub fn run() -> Result<(), Box<dyn Error>> {
  let cli = Cli::parse();
  run_with_cli(cli)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::path::PathBuf;
  use tempfile::tempdir;

  #[test]
  fn cli_parse_top_level_regent_command() {
    let cli = Cli::try_parse_from(["swini", "regent", "start"]).unwrap();
    assert_eq!(
      cli.command,
      Command::Regent {
        command: RegentCommand::Start {
          config: None,
          detached: false,
        }
      }
    );

    let cli = Cli::try_parse_from([
      "swini",
      "regent",
      "start",
      "-c",
      "custom.yml",
      "-d",
    ])
    .unwrap();
    assert_eq!(
      cli.command,
      Command::Regent {
        command: RegentCommand::Start {
          config: Some(PathBuf::from("custom.yml")),
          detached: true,
        }
      }
    );
  }

  #[test]
  fn cli_parse_top_level_status_and_croft_commands() {
    let cli = Cli::try_parse_from(["swini", "status"]).unwrap();
    assert_eq!(cli.command, Command::Status);

    let cli =
      Cli::try_parse_from(["swini", "croft", "status", "node-1", "--live"])
        .unwrap();
    assert_eq!(
      cli.command,
      Command::Croft {
        command: CroftCommand::Status {
          name: "node-1".to_string(),
          live: true,
        }
      }
    );
  }

  #[test]
  fn resolve_api_url_resolution() {
    let _lock = crate::core::TEST_ENV_MUTEX.lock().unwrap();

    // 1. Default URL (no env)
    std::env::remove_var("SWINI_ADDR");
    let url = resolve_api_url().unwrap();
    assert_eq!(url, "http://127.0.0.1:7440");

    // 2. SWINI_ADDR override without scheme
    std::env::set_var("SWINI_ADDR", "127.0.0.1:8888");
    let url = resolve_api_url().unwrap();
    assert_eq!(url, "http://127.0.0.1:8888");

    // 3. SWINI_ADDR with http scheme
    std::env::set_var("SWINI_ADDR", "http://127.0.0.1:9999");
    let url = resolve_api_url().unwrap();
    assert_eq!(url, "http://127.0.0.1:9999");

    // 4. SWINI_ADDR with https scheme
    std::env::set_var("SWINI_ADDR", "https://example.com:9999");
    let url = resolve_api_url().unwrap();
    assert_eq!(url, "https://example.com:9999");

    // 5. Invalid local:// URL
    std::env::set_var("SWINI_ADDR", "local://nonexistent-croft-404");
    assert!(resolve_api_url().is_err());

    // 6. local:// resolution from state
    let dir = tempdir().unwrap();
    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", dir.path());
    let data_dir = config::default_data_dir("test-croft");

    let state = crate::regent::RegentState {
      pid: std::process::id(),
      detached: false,
      started_at: "2026-09-03T15:00:00Z".to_string(),
      config: crate::croft::Config {
        name: "test-croft".to_string(),
        addr: "127.0.0.1:7555".parse().unwrap(),
        data_dir,
        ..Default::default()
      },
    };
    state.save().unwrap();

    std::env::set_var("SWINI_ADDR", "local://test-croft");
    let local_uri_resolved = resolve_api_url();
    assert!(local_uri_resolved.is_ok());
    assert_eq!(local_uri_resolved.unwrap(), "http://127.0.0.1:7555");

    if let Some(h) = original_home {
      std::env::set_var("HOME", h);
    }
    std::env::remove_var("SWINI_ADDR");

    // 7. Calling resolve_endpoint directly
    let endpoint = resolve_endpoint();
    assert!(endpoint.is_ok());
  }

  #[test]
  fn cli_parse_top_level_pig_commands() {
    let cli =
      Cli::try_parse_from(["swini", "pig", "run", "-c", "pigs.yml", "api"])
        .unwrap();
    assert_eq!(
      cli.command,
      Command::Pig {
        command: PigCommand::Run {
          config: Some(PathBuf::from("pigs.yml")),
          names: vec!["api".to_string()],
        }
      }
    );

    let cli_stop =
      Cli::try_parse_from(["swini", "pig", "stop", "api"]).unwrap();
    assert_eq!(
      cli_stop.command,
      Command::Pig {
        command: PigCommand::Stop {
          config: None,
          name: "api".to_string(),
        }
      }
    );
  }
}
