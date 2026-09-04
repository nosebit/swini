//! Swini Regent lifecycle management, process supervision, and cluster
//! coordination.
//!
//! Submodules:
//! - [`state`]: Manages runtime process state persistence (`state.json`).

pub mod state;

pub use state::RegentState;

use crate::core::config::{self, Config, DEFAULT_NAME};
use crate::core::proto::plot::plot_api_client::PlotApiClient;
use crate::core::proto::plot::JoinReq;
use crate::core::telemetry;
use crate::croft::Croft;
use crate::plot::clerk::api::plot_from_join_req;
use crate::plot::{Clerk as PlotClerk, Plot};
use crate::store::barn::Node as BarnNode;
use crate::store::{SpreadNode, SpreadStore};
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

/// Starts the Swini Regent runtime.
///
/// In foreground mode (default), logs are streamed to stdout and the process
/// blocks on shutdown signals (`SIGINT`/`SIGTERM`). In detached mode (`detached
/// = true`), the process forks into the background via
/// [`daemonize::Daemonize`].
pub async fn start(
  config_path: Option<PathBuf>,
  detached: bool,
) -> Result<(), Box<dyn Error>> {
  let config = Config::load(config_path.as_deref())?;

  let log_dir = config.data_dir.join("logs");
  let _telemetry_guard =
    telemetry::init(!detached, Some(&log_dir), Some(&config.name));

  tracing::info!(name = %config.name, addr = %config.addr, "starting swini regent");

  let state = RegentState {
    pid: std::process::id(),
    detached,
    started_at: chrono::Utc::now().to_rfc3339(),
    config: config.clone(),
  };
  state.save()?;

  let croft = Arc::new(Croft::spawn(&config).await?);

  if !config.join_addresses.is_empty() {
    let _ = bootstrap_join(&croft).await;
  }

  let _plot_clerk = PlotClerk::spawn(croft.clone())?;

  let bind_addr = config.addr;
  let server_handle = tokio::spawn({
    let croft = croft.clone();
    async move {
      if let Err(e) = croft.gate.listen(bind_addr).await {
        tracing::error!(error = %e, "Gate server terminated unexpectedly");
      }
    }
  });

  let mut sigterm =
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
  tokio::select! {
    _ = tokio::signal::ctrl_c() => {
      tracing::info!("received SIGINT, shutting down");
    }
    _ = sigterm.recv() => {
      tracing::info!("received SIGTERM, shutting down");
    }
    _ = server_handle => {
      tracing::error!("Gate server task exited");
    }
  }

  RegentState::cleanup(&config.data_dir);
  tracing::info!("regent stopped cleanly");
  std::process::exit(0);
}

/// Returns candidate data directories where state.json may reside.
fn candidate_data_dirs(name: &str) -> Vec<PathBuf> {
  vec![
    config::default_data_dir(name),
    PathBuf::from("sandbox/.data").join(name),
    PathBuf::from(".data").join(name),
  ]
}

/// Gracefully terminates a running Regent instance by PID.
pub async fn stop(name: Option<String>) -> Result<(), Box<dyn Error>> {
  let target_name = name.as_deref().unwrap_or(DEFAULT_NAME);
  let candidates = candidate_data_dirs(target_name);
  let mut resolved = None;

  for dir in candidates {
    if let Ok(state) = RegentState::load(&dir) {
      resolved = Some((dir, state));
      break;
    }
  }

  let (data_dir, state) = match resolved {
    Some(pair) => pair,
    None => {
      return Err(
        format!("Regent '{}' is not running (no state found)", target_name)
          .into(),
      );
    }
  };

  if !state.is_alive() {
    RegentState::cleanup(&data_dir);
    println!(
      "Regent '{}' was not running (cleaned up stale state).",
      target_name
    );
    return Ok(());
  }

  println!("Stopping regent '{}' (PID {})...", target_name, state.pid);
  unsafe {
    libc::kill(state.pid as i32, libc::SIGTERM);
  }

  for _ in 0..50 {
    if !state.is_alive() {
      RegentState::cleanup(&data_dir);
      println!("Regent '{}' stopped successfully.", target_name);
      return Ok(());
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
  }

  println!("Regent '{}' did not terminate in time.", target_name);
  Ok(())
}

/// Inspects and displays the status of local Regent instances.
pub async fn status(name: Option<String>) -> Result<(), Box<dyn Error>> {
  let mut roots = vec![config::regents_dir()];
  let sandbox_dir = PathBuf::from("sandbox/.data");
  if sandbox_dir.exists() {
    roots.push(sandbox_dir);
  }
  let dot_data_dir = PathBuf::from(".data");
  if dot_data_dir.exists() {
    roots.push(dot_data_dir);
  }

  let mut seen = std::collections::HashSet::new();
  let mut header_printed = false;

  for regents_root in roots {
    if !regents_root.exists() {
      continue;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(regents_root).await {
      while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
          let regent_name = entry.file_name().to_string_lossy().to_string();
          if seen.contains(&regent_name) {
            continue;
          }
          if let Some(ref target) = name {
            if target != &regent_name {
              continue;
            }
          }

          if let Ok(state) = RegentState::load(&entry.path()) {
            if !header_printed {
              println!(
                "{:<15} {:<8} {:<22} {:<10} {:<20}",
                "NAME", "PID", "ADDR", "STATUS", "STARTED AT"
              );
              println!(
                "{:-<15} {:-<8} {:-<22} {:-<10} {:-<20}",
                "", "", "", "", ""
              );
              header_printed = true;
            }

            let status_str = if state.is_alive() {
              "Running"
            } else {
              "Dead (Stale)"
            };
            println!(
              "{:<15} {:<8} {:<22} {:<10} {:<20}",
              state.config.name,
              state.pid,
              state.config.addr,
              status_str,
              state.started_at
            );
            seen.insert(regent_name);
          }
        }
      }
    }
  }

  if !header_printed {
    if name.is_some() {
      println!("No matching regents found.");
    } else {
      println!("No regents running.");
    }
  }
  Ok(())
}

async fn dial_join(
  peer_addr: &str,
  plot: &Plot,
) -> Result<Vec<Plot>, Box<dyn Error>> {
  let endpoint =
    if peer_addr.starts_with("http://") || peer_addr.starts_with("https://") {
      peer_addr.to_string()
    } else {
      format!("http://{}", peer_addr)
    };

  let mut client = PlotApiClient::connect(endpoint).await?;
  let req = JoinReq {
    id: plot.id,
    name: plot.name.clone(),
    addr: plot.addr.clone(),
    roles: plot.roles.iter().map(|r| r.to_string()).collect(),
    tags: plot.tags.clone(),
  };

  let res = client.join(req).await?.into_inner();
  let server_plots = res
    .server_plots
    .into_iter()
    .filter_map(|p| {
      plot_from_join_req(JoinReq {
        id: p.id,
        name: p.name,
        addr: p.addr,
        roles: p.roles,
        tags: p.tags,
      })
      .ok()
    })
    .collect();

  Ok(server_plots)
}

/// Dials configured peer join addresses to register this Plot and cache active
/// Server plots.
pub async fn bootstrap_join(croft: &Croft) -> Result<(), Box<dyn Error>> {
  for peer in &croft.config.join_addresses {
    match dial_join(peer, &croft.plot).await {
      Ok(server_plots) => {
        if !croft.plot.is_server() {
          let barn_nodes: Vec<BarnNode> = server_plots
            .into_iter()
            .map(|p| BarnNode::new(p.id, p.addr))
            .collect();
          croft.barn.node_cache_set(barn_nodes).await?;
        }
        return Ok(());
      }
      Err(e) => {
        tracing::warn!(peer = %peer, error = %e, "failed to join peer, trying next");
      }
    }
  }

  Err("Failed to join ranch from any configured peer address".into())
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[tokio::test]
  async fn bootstrap_join_all_unreachable_returns_error() {
    let dir = tempdir().unwrap();
    let addr: std::net::SocketAddr = "127.0.0.1:7440".parse().unwrap();
    let config = Config {
      name: "test-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      join_addresses: vec!["127.0.0.1:1".to_string()],
      ..Default::default()
    };
    let croft = Croft::spawn(&config).await.unwrap();

    let res = bootstrap_join(&croft).await;
    assert!(res.is_err());
  }
}
