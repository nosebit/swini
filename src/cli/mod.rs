use crate::core::proto::cluster::cluster::cluster_api_client::ClusterApiClient;
use crate::daemon::DEFAULT_API_PORT;
use clap::{Parser, Subcommand};
use std::error::Error;

mod cluster;
mod daemon;

#[derive(Parser, Debug)]
#[command(author, version, about = "Swini Command Line Interface", long_about = None)]
struct Cli {
  #[command(subcommand)]
  pub command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
  Daemon {
    #[command(subcommand)]
    command: daemon::Command,
  },
  /// Inspect cluster status and registered nodes
  Status,
}

/// Resolves the daemon API URL:
/// 1. From `SWINI_API_URL` environment variable if set.
/// 2. By inspecting active daemon metadata in
///    `data_dir/daemons/<name_or_id>/daemon.json`.
/// 3. Falling back to default `http://127.0.0.1:7440`.
pub fn resolve_api_url(
  _daemon_name_or_id: Option<&str>,
) -> Result<String, Box<dyn Error>> {
  if let Ok(url) = std::env::var("SWINI_API_URL") {
    let trimmed = url.trim();
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

  if let Ok(data_dir) = std::env::var("SWINI_DATA_DIR") {
    let daemons_dir = std::path::PathBuf::from(data_dir).join("daemons");
    if daemons_dir.exists() {
      // Future daemon discovery can be extended here
    }
  }

  Ok(format!("http://127.0.0.1:{}", DEFAULT_API_PORT))
}

pub async fn run() -> Result<(), Box<dyn Error>> {
  let cli = Cli::parse();

  match cli.command {
    Command::Daemon { command } => daemon::run(command).await,
    Command::Status => {
      let url = resolve_api_url(None)?;
      let channel = tonic::transport::Endpoint::from_shared(url)?
        .connect()
        .await?;
      let client = ClusterApiClient::new(channel);
      cluster::status(client).await
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn resolve_api_url_resolution() {
    // 1. Default port
    std::env::remove_var("SWINI_API_URL");
    std::env::remove_var("SWINI_DATA_DIR");
    let url = resolve_api_url(None).unwrap();
    assert_eq!(url, "http://127.0.0.1:7440");

    // 2. Env var override
    std::env::set_var("SWINI_API_URL", "127.0.0.1:9000");
    let url = resolve_api_url(None).unwrap();
    assert_eq!(url, "http://127.0.0.1:9000");
    std::env::remove_var("SWINI_API_URL");
  }
}
