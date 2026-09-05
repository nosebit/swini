//! Operational compound representing the physical farmstead on a single host
//! machine, uniting local domain [`Plot`] identity, persistent [`Barn`]
//! storage, network [`Gate`], and active [`Config`].
//!
//! Exposes [`Croft`], which provides direct access to the foundational
//! infrastructure and acts as the operational base where domain staff members
//! (such as the `PlotClerk`) are hired.

pub mod gate;

pub use gate::Gate;

use crate::core::Config;
use crate::plot::Plot;
use crate::store::barn::{Barn, Config as BarnConfig};
use std::error::Error;
use std::sync::Arc;

/// Operational compound representing a self-contained farmstead on a single
/// machine, holding its domain [`Plot`] land, persistent [`Barn`] storage silo,
/// network [`Gate`], and active [`Config`].
pub struct Croft {
  /// Pure domain identity and metadata for this local Plot.
  pub plot: Plot,
  /// Consensus replicated key-value storage engine.
  pub barn: Arc<Barn>,
  /// Network gRPC gateway for service registration and listening.
  pub gate: Gate,
  /// Active runtime configuration.
  pub config: Config,
}

impl Croft {
  /// Spawns the local [`Croft`] compound, initializing the local [`Plot`]
  /// identity, network [`Gate`], and [`Barn`] storage engine.
  pub async fn spawn(config: &Config) -> Result<Self, Box<dyn Error>> {
    let gate = Gate::new();
    let plot = Plot::new(config)?;
    let barn = Arc::new(
      Barn::spawn(
        &gate,
        BarnConfig {
          id: plot.id,
          addr: config.addr.to_string(),
          data_dir: config.data_dir.join("data"),
          heartbeat_interval: 100,
          election_timeout_min: 300,
          election_timeout_max: 600,
        },
      )
      .await?,
    );

    Ok(Self {
      plot,
      barn,
      gate,
      config: config.clone(),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::net::SocketAddr;
  use tempfile::tempdir;

  #[tokio::test]
  async fn croft_spawns_cleanly() {
    let dir = tempdir().unwrap();
    let addr: SocketAddr = "127.0.0.1:7440".parse().unwrap();
    let config = Config {
      name: "test-node".to_string(),
      addr,
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };

    let croft = Croft::spawn(&config).await.unwrap();
    assert_eq!(croft.plot.name, "test-node");
    assert!(croft.plot.id > 0);
  }
}
