//! Swini Regent lifecycle management, process supervision, and cluster
//! coordination.
//!
//! Submodules:
//! - [`state`]: Manages runtime process state persistence (`regent.json`).

pub mod state;

pub use state::RegentState;

use crate::core::proto::croft::croft_api_client::CroftApiClient;
use crate::core::proto::croft::JoinReq;
use crate::core::telemetry;
use crate::croft::config::{self, Config, DEFAULT_NAME};
use crate::croft::{Clerk as CroftClerk, Croft, LiveCroft};
use crate::store::barn::Node as BarnNode;
use crate::store::{SpreadNode, SpreadStore};
use std::collections::BTreeMap;
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

  let croft = Arc::new(LiveCroft::spawn(&config).await?);

  if config.join_addresses.is_empty() && croft.is_server() {
    let self_node = BarnNode::new(croft.id, croft.addr.clone());
    let mut members = BTreeMap::new();
    members.insert(self_node.id, self_node);
    let _ = croft.barn.raft().initialize(members).await;
  } else if !config.join_addresses.is_empty() {
    let _ = bootstrap_join(&config, &croft).await;
  }

  let _croft_clerk = CroftClerk::spawn(croft.clone())?;

  // Persist the croft info into the Barn
  let _ = croft.persist().await;

  let bind_addr = config.addr;
  let gate = croft.gate.clone();
  let server_handle = tokio::spawn(async move {
    if let Err(e) = gate.listen(bind_addr).await {
      tracing::error!(error = %e, "Gate server terminated unexpectedly");
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

/// Returns candidate data directories where regent.json may reside.
fn candidate_data_dirs(name: &str) -> Vec<PathBuf> {
  vec![
    config::default_data_dir(name),
    PathBuf::from("sandbox/.data").join(name),
    PathBuf::from(".data").join(name),
  ]
}

/// Gracefully terminates a running Regent instance by PID.
///
/// # Errors
/// Returns an error if the named Regent state cannot be located.
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
///
/// # Errors
/// Returns an error if directory traversal fails.
pub async fn status(name: Option<String>) -> Result<(), Box<dyn Error>> {
  let mut roots = vec![config::crofts_dir()];
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

  for crofts_root in roots {
    if !crofts_root.exists() {
      continue;
    }

    if let Ok(mut entries) = tokio::fs::read_dir(crofts_root).await {
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
  croft: &Croft,
) -> Result<Vec<Croft>, Box<dyn Error>> {
  let endpoint =
    if peer_addr.starts_with("http://") || peer_addr.starts_with("https://") {
      peer_addr.to_string()
    } else {
      format!("http://{}", peer_addr)
    };

  let mut client = CroftApiClient::connect(endpoint).await?;
  let req = JoinReq {
    id: croft.id,
    name: croft.name.clone(),
    addr: croft.addr.clone(),
    roles: croft.roles.iter().map(|r| r.to_string()).collect(),
    tags: croft.tags.clone(),
    resources: Some(croft.resources.clone().into()),
  };

  let res = client.join(req).await?.into_inner();
  let server_crofts = res
    .server_crofts
    .into_iter()
    .filter_map(|c| Croft::try_from(c).ok())
    .collect();

  Ok(server_crofts)
}

/// Dials configured peer join addresses to register this Croft and cache active
/// Server crofts.
///
/// # Errors
/// Returns an error if none of the configured peer addresses could be
/// contacted.
pub async fn bootstrap_join(
  config: &Config,
  croft: &LiveCroft,
) -> Result<(), Box<dyn Error>> {
  for peer in &config.join_addresses {
    match dial_join(peer, croft).await {
      Ok(server_crofts) => {
        if !croft.is_server() {
          let barn_nodes: Vec<BarnNode> = server_crofts
            .into_iter()
            .map(|c| BarnNode::new(c.id, c.addr))
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
  use crate::croft::CroftRole;
  use tempfile::tempdir;

  #[tokio::test]
  async fn bootstrap_join_all_unreachable_returns_error() {
    let dir = tempfile::tempdir().unwrap();
    let addr: std::net::SocketAddr = "127.0.0.1:7440".parse().unwrap();
    let config = Config {
      name: "test-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      join_addresses: vec!["127.0.0.1:1".to_string()],
      ..Default::default()
    };
    let croft = LiveCroft::spawn(&config).await.unwrap();

    let res = bootstrap_join(&config, &croft).await;
    assert!(res.is_err());
  }

  #[tokio::test]
  async fn bootstrap_join_success_populates_barn_node_cache() {
    let dir_server = tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let server_addr = listener.local_addr().unwrap();
    drop(listener);

    let server_config = Config {
      name: "server-node".to_string(),
      addr: server_addr,
      data_dir: dir_server.path().to_path_buf(),
      roles: vec![CroftRole::Server],
      ..Default::default()
    };
    let server_croft =
      Arc::new(LiveCroft::spawn(&server_config).await.unwrap());
    let self_node = BarnNode::new(server_croft.id, server_croft.addr.clone());
    let mut members = std::collections::BTreeMap::new();
    members.insert(self_node.id, self_node);
    server_croft.barn.raft().initialize(members).await.unwrap();

    let server_clerk = CroftClerk::spawn(server_croft.clone()).unwrap();
    let server_croft_base = Croft {
      id: server_croft.id,
      name: server_croft.name.clone(),
      addr: server_croft.addr.clone(),
      roles: server_croft.roles.clone(),
      tags: server_croft.tags.clone(),
      joined_at: String::new(),
      resources: server_croft.resources.clone(),
    };
    let _ = server_clerk.join(server_croft_base).await.unwrap();

    let gate_server = server_croft.gate.clone();
    let server_handle = tokio::spawn(async move {
      let _ = gate_server.listen(server_addr).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Worker node joins the server
    let dir_worker = tempdir().unwrap();
    let listener_w =
      tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let worker_addr = listener_w.local_addr().unwrap();
    drop(listener_w);

    let worker_config = Config {
      name: "worker-node".to_string(),
      addr: worker_addr,
      data_dir: dir_worker.path().to_path_buf(),
      roles: vec![CroftRole::Worker],
      join_addresses: vec![format!("http://{}", server_addr)],
      ..Default::default()
    };
    let worker_croft = LiveCroft::spawn(&worker_config).await.unwrap();

    let join_res =
      dial_join(&format!("http://{}", server_addr), &worker_croft).await;
    assert!(join_res.is_ok());

    let server_crofts = join_res.unwrap();
    assert!(!server_crofts.is_empty());

    server_handle.abort();
  }

  #[tokio::test]
  async fn candidate_data_dirs_and_status_queries() {
    let dirs = candidate_data_dirs("test-instance");
    assert_eq!(dirs.len(), 3);

    // Query non-existent status
    let res = status(Some("definitely-nonexistent-regent".to_string())).await;
    assert!(res.is_ok());

    let res_all = status(None).await;
    assert!(res_all.is_ok());
  }
}
