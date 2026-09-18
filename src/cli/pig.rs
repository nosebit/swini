//! Workload deployment and lifecycle CLI command handlers.
//!
//! Exposes [`Command`] for the `swini pig` subcommand suite (`run`, `stop`),
//! reading declarative YAML specifications, connecting to the target Croft's
//! [`PigApi`](crate::core::proto::pig::pig_api_client::PigApiClient) gRPC
//! service, and displaying execution outcomes.

use crate::core::proto::pig::pig_api_client::PigApiClient;
use crate::core::proto::pig::{PigRunReq, PigStopReq};
use crate::pig::config::PigRunConfig;
use clap::Subcommand;
use std::collections::HashSet;
use std::error::Error;
use std::path::PathBuf;

/// Workload lifecycle subcommands under `swini pig`.
#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Command {
  /// Deploys and starts pigs defined in a configuration file
  Run {
    /// Path to configuration file (defaults to pigs.yml)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Optional specific pig names to run
    #[arg(trailing_var_arg = true)]
    names: Vec<String>,
  },
  /// Stops a running pig workload
  Stop {
    /// Path to configuration file (defaults to pigs.yml)
    #[arg(short, long)]
    config: Option<PathBuf>,
    /// Name or path of the pig to stop
    name: String,
  },
}

/// Dispatches `swini pig` commands.
///
/// # Errors
/// Returns an error if the config file cannot be loaded, contains invalid pigs,
/// or if gRPC communication with the Croft Gate fails.
pub async fn run(
  cmd: Command,
  endpoint: tonic::transport::Endpoint,
) -> Result<(), Box<dyn Error>> {
  let mut client = PigApiClient::connect(endpoint).await?;

  match cmd {
    Command::Run {
      config: config_path,
      names,
    } => {
      let path = config_path.unwrap_or_else(|| PathBuf::from("pigs.yml"));
      let mut config = PigRunConfig::load(&path)?;

      if !names.is_empty() {
        let name_set: HashSet<_> = names.iter().collect();
        // Validate that all specified pig names exist in the configuration
        for req_name in &names {
          if !config.pigs.iter().any(|p| &p.name == req_name) {
            return Err(
              format!("Pig '{}' not found in configuration", req_name).into(),
            );
          }
        }
        config.pigs.retain(|p| name_set.contains(&p.name));
      }

      let req = PigRunReq {
        config_yaml: serde_yml::to_string(&config)?,
      };
      let res = client.run(req).await?.into_inner();
      println!("{}", res.status);
      for pig_path in res.started_pigs {
        println!("Started {}", pig_path);
      }
    }
    Command::Stop {
      config: config_path,
      name,
    } => {
      let space = if let Some(path) = config_path {
        PigRunConfig::load(&path)
          .map(|m| m.space)
          .unwrap_or_else(|_| "main".to_string())
      } else {
        "main".to_string()
      };

      let req = PigStopReq { space, name };
      let res = client.stop(req).await?.into_inner();
      println!("{}", res.status);
    }
  }

  Ok(())
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
  fn test_parse_pig_run_args() {
    let cli = TestCli::try_parse_from([
      "pig",
      "run",
      "-c",
      "custom.yml",
      "worker",
      "api",
    ])
    .unwrap();
    assert_eq!(
      cli.command,
      Command::Run {
        config: Some(PathBuf::from("custom.yml")),
        names: vec!["worker".to_string(), "api".to_string()],
      }
    );

    let cli_default = TestCli::try_parse_from(["pig", "run"]).unwrap();
    assert_eq!(
      cli_default.command,
      Command::Run {
        config: None,
        names: vec![],
      }
    );
  }

  #[test]
  fn test_parse_pig_stop_args() {
    let cli = TestCli::try_parse_from(["pig", "stop", "api-gateway"]).unwrap();
    assert_eq!(
      cli.command,
      Command::Stop {
        config: None,
        name: "api-gateway".to_string(),
      }
    );
  }
}
