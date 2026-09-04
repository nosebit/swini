//! Command-line interface definitions and dispatch for Swini.
//!
//! Exposes top-level command routing via [`Cli`] and [`run`], along with
//! endpoint address resolution in [`resolve_api_url`].
//!
//! Submodules:
//! - [`regent`]: Scoped subcommands for managing local Regent lifecycle.

mod regent;

pub(crate) use regent::Command as RegentCommand;

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
///    `~/.swini/regents/{name}/state.json`.
/// 3. Falling back to default `http://127.0.0.1:7440`.
pub fn resolve_api_url(
  regent_name: Option<&str>,
) -> Result<String, Box<dyn Error>> {
  let find_state = |name: &str| -> Option<crate::regent::RegentState> {
    let candidates = [
      crate::core::config::default_data_dir(name),
      PathBuf::from("sandbox/.data").join(name),
      PathBuf::from(".data").join(name),
    ];
    for dir in candidates {
      if let Ok(state) = crate::regent::RegentState::load(&dir) {
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

  Ok(format!("http://{}", crate::core::config::DEFAULT_ADDR))
}

/// Dispatches pre-parsed CLI commands inside an active async runtime.
async fn dispatch(cli: Cli) -> Result<(), Box<dyn Error>> {
  match cli.command {
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
  fn resolve_api_url_resolution() {
    std::env::remove_var("SWINI_ADDR");

    // 1. Default URL
    let url = resolve_api_url(None).unwrap();
    assert_eq!(url, "http://127.0.0.1:7440");

    // 2. SWINI_ADDR override
    std::env::set_var("SWINI_ADDR", "127.0.0.1:8888");
    let url = resolve_api_url(None).unwrap();
    assert_eq!(url, "http://127.0.0.1:8888");

    // 3. SWINI_ADDR with http scheme
    std::env::set_var("SWINI_ADDR", "http://127.0.0.1:9999");
    let url = resolve_api_url(None).unwrap();
    assert_eq!(url, "http://127.0.0.1:9999");
    std::env::remove_var("SWINI_ADDR");

    // 4. local:// resolution from state
    let dir = tempdir().unwrap();
    let state = crate::regent::RegentState {
      pid: std::process::id(),
      detached: false,
      started_at: "2026-09-03T15:00:00Z".to_string(),
      config: crate::core::Config {
        name: "test-regent".to_string(),
        addr: "127.0.0.1:7555".parse().unwrap(),
        data_dir: dir.path().to_path_buf(),
        ..Default::default()
      },
    };
    state.save().unwrap();

    let resolved = resolve_api_url(Some("test-regent"));
    assert!(resolved.is_ok());
  }
}
