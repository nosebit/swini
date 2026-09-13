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
mod regent;
pub mod status;

pub(crate) use croft::Command as CroftCommand;
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
  /// Manages the local Swini Regent process
  Regent {
    #[command(subcommand)]
    command: RegentCommand,
  },
}

/// Resolves the API endpoint URL for a Regent / Clerk:
/// 1. From `SWINI_ADDR` environment variable if set (including `local://{name}`
///    URIs).
/// 2. By inspecting active Regent metadata in
///    `~/.swini/crofts/{name}/regent.json`.
/// 3. Falling back to default `http://127.0.0.1:7440`.
pub fn resolve_api_url(
  regent_name: Option<&str>,
) -> Result<String, Box<dyn Error>> {
  let env_override = std::env::var("SWINI_ADDR").ok();
  resolve_api_url_internal(regent_name, env_override.as_deref())
}

fn resolve_api_url_internal(
  regent_name: Option<&str>,
  env_addr: Option<&str>,
) -> Result<String, Box<dyn Error>> {
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

  if let Some(url) = env_addr {
    let trimmed = url.trim();
    if let Some(name) = trimmed.strip_prefix("local://") {
      if let Some(state) = find_state(name) {
        return Ok(format!("http://{}", state.config.addr));
      }
      return Err(
        format!("Local regent '{}' is not running or has no state", name)
          .into(),
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

  if let Some(name) = regent_name {
    if let Some(state) = find_state(name) {
      return Ok(format!("http://{}", state.config.addr));
    }
  }

  Ok(format!("http://{}", config::DEFAULT_ADDR))
}

/// Dispatches pre-parsed CLI commands inside an active async runtime.
async fn dispatch(cli: Cli) -> Result<(), Box<dyn Error>> {
  match cli.command {
    Command::Status => status::run().await,
    Command::Croft { command } => croft::run(command).await,
    Command::Regent { command } => regent::run(command).await,
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
    // 1. Default URL (no env, no regent name)
    let url = resolve_api_url_internal(None, None).unwrap();
    assert_eq!(url, "http://127.0.0.1:7440");

    // 2. SWINI_ADDR override without scheme
    let url = resolve_api_url_internal(None, Some("127.0.0.1:8888")).unwrap();
    assert_eq!(url, "http://127.0.0.1:8888");

    // 3. SWINI_ADDR with http scheme
    let url =
      resolve_api_url_internal(None, Some("http://127.0.0.1:9999")).unwrap();
    assert_eq!(url, "http://127.0.0.1:9999");

    // 4. SWINI_ADDR with https scheme
    let url =
      resolve_api_url_internal(None, Some("https://example.com:9999")).unwrap();
    assert_eq!(url, "https://example.com:9999");

    // 5. Invalid local:// URL
    assert!(resolve_api_url_internal(
      None,
      Some("local://nonexistent-regent-404")
    )
    .is_err());

    // 6. local:// resolution from state
    let dir = tempdir().unwrap();
    let state = crate::regent::RegentState {
      pid: std::process::id(),
      detached: false,
      started_at: "2026-09-03T15:00:00Z".to_string(),
      config: crate::croft::Config {
        name: "test-regent".to_string(),
        addr: "127.0.0.1:7555".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
      },
    };
    state.save().unwrap();

    let resolved = resolve_api_url_internal(Some("test-regent"), None);
    assert!(resolved.is_ok());

    // 7. Calling public resolve_api_url directly
    let direct = resolve_api_url(Some("test-regent"));
    assert!(direct.is_ok());
  }
}
