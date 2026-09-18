//! Croft configuration loading, YAML merging, and filesystem path resolution.
//!
//! Exposes [`Config`] and its resolution hierarchy:
//! 1. Base built-in defaults (`DEFAULT_NAME`, `DEFAULT_ADDR`, default roles).
//! 2. Local YAML file overrides (`./swini.yml` or explicitly specified
//!    `--config <path>`).
//! 3. Environment variable overrides (`SWINI_ADDR`, `SWINI_DATA_DIR`).
//!
//! All instance data is isolated under [`default_data_dir(name)`]
//! (`~/.swini/crofts/{name}`).

use crate::croft::resources::CroftResources;
use crate::croft::types::CroftRole;
use figment::{
  providers::{Env, Format, Serialized, Yaml},
  Figment,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Hardware capacity configuration override for a Croft in MHz and MB.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ResourcesConfig {
  /// Total physical or configured CPU capacity in MHz (or parsed from unit
  /// strings like `"4 GHz"`, `"4000 MHz"`).
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_cpu_mhz"
  )]
  pub cpu: Option<f64>,
  /// Schedulable CPU capacity in MHz. Defaults to `cpu` if omitted.
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_cpu_mhz"
  )]
  pub cpu_yardable: Option<f64>,
  /// Total physical or configured RAM capacity in MB (or parsed from unit
  /// strings like `"8 GB"`, `"8000 MB"`).
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_memory_mb"
  )]
  pub mem: Option<f64>,
  /// Schedulable RAM capacity in MB. Defaults to `mem` if omitted.
  #[serde(
    default,
    deserialize_with = "crate::core::format::deserialize_opt_memory_mb"
  )]
  pub mem_yardable: Option<f64>,
}

impl ResourcesConfig {
  /// Converts configured resource overrides into [`CroftResources`],
  /// falling back to hardware probing when fields are omitted.
  pub fn to_resources(&self) -> CroftResources {
    let probed = CroftResources::probe();
    let cpu_total = self.cpu.unwrap_or(probed.cpu_total);
    let cpu_yardable = self.cpu_yardable.unwrap_or(if self.cpu.is_some() {
      cpu_total
    } else {
      probed.cpu_yardable
    });
    let mem_total = self.mem.unwrap_or(probed.mem_total);
    let mem_yardable = self.mem_yardable.unwrap_or(if self.mem.is_some() {
      mem_total
    } else {
      probed.mem_yardable
    });

    CroftResources {
      cpu_total,
      cpu_yardable,
      cpu_reserved: 0.0,
      mem_total,
      mem_yardable,
      mem_reserved: 0.0,
    }
  }
}

/// Default socket address where the Croft's Gate and Clerk listen for incoming
/// Ranch traffic.
pub const DEFAULT_ADDR: &str = "127.0.0.1:7440";

/// Default instance name for the local Croft and Regent runtime process.
pub const DEFAULT_NAME: &str = "main";

/// Default timeout for establishing TCP/TLS connection.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for executing a single gRPC request.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// Default timeout for contacting candidate peers during bootstrap join.
pub const DEFAULT_JOIN_TIMEOUT: Duration = Duration::from_secs(3);

/// Default timeout for internal Barn consensus Raft RPCs.
pub const DEFAULT_RAFT_TIMEOUT: Duration = Duration::from_secs(2);

fn default_connect_timeout() -> Duration {
  DEFAULT_CONNECT_TIMEOUT
}

fn default_request_timeout() -> Duration {
  DEFAULT_REQUEST_TIMEOUT
}

fn default_join_timeout() -> Duration {
  DEFAULT_JOIN_TIMEOUT
}

fn default_raft_timeout() -> Duration {
  DEFAULT_RAFT_TIMEOUT
}

/// Network and consensus operation timeout configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeoutsConfig {
  /// Timeout for establishing TCP/TLS connections to remote Crofts and Clerks.
  #[serde(
    default = "default_connect_timeout",
    serialize_with = "crate::core::format::serialize_duration",
    deserialize_with = "crate::core::format::deserialize_duration"
  )]
  pub connect: Duration,

  /// Overall timeout for individual gRPC request execution.
  #[serde(
    default = "default_request_timeout",
    serialize_with = "crate::core::format::serialize_duration",
    deserialize_with = "crate::core::format::deserialize_duration"
  )]
  pub request: Duration,

  /// Timeout when contacting candidate peers during cluster bootstrap join.
  #[serde(
    default = "default_join_timeout",
    serialize_with = "crate::core::format::serialize_duration",
    deserialize_with = "crate::core::format::deserialize_duration"
  )]
  pub join: Duration,

  /// Timeout for internal Barn Raft RPCs (AppendEntries, RequestVote).
  #[serde(
    default = "default_raft_timeout",
    serialize_with = "crate::core::format::serialize_duration",
    deserialize_with = "crate::core::format::deserialize_duration"
  )]
  pub raft: Duration,
}

impl Default for TimeoutsConfig {
  fn default() -> Self {
    Self {
      connect: default_connect_timeout(),
      request: default_request_timeout(),
      join: default_join_timeout(),
      raft: default_raft_timeout(),
    }
  }
}

/// Runtime configuration for a Swini Croft and its supervising Regent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
  /// Name identifying this Croft instance (e.g., "worker-01").
  #[serde(default = "default_name")]
  pub name: String,

  /// Canonical network address where this Croft's Gate listens.
  #[serde(default = "default_addr")]
  pub addr: SocketAddr,

  /// Root directory on the local filesystem for runtime state, Barn consensus
  /// data, and logs.
  #[serde(default)]
  pub data_dir: PathBuf,

  /// Operational roles assigned to this Croft (Server, Worker).
  #[serde(default = "default_roles")]
  pub roles: Vec<CroftRole>,

  /// User-defined tags and capability metadata associated with this Croft.
  #[serde(default)]
  pub tags: Vec<String>,

  /// Optional hardware resource overrides (CPU, RAM).
  #[serde(default)]
  pub resources: Option<ResourcesConfig>,

  /// Bootstrap peer addresses to contact when joining an existing Ranch
  /// cluster.
  #[serde(default)]
  pub join_addresses: Vec<String>,

  /// Timeout configuration for network and consensus operations.
  #[serde(default)]
  pub timeouts: TimeoutsConfig,
}

fn default_name() -> String {
  DEFAULT_NAME.to_string()
}

fn default_addr() -> SocketAddr {
  DEFAULT_ADDR
    .parse()
    .expect("DEFAULT_ADDR must be a valid SocketAddr")
}

fn default_roles() -> Vec<CroftRole> {
  vec![CroftRole::Server, CroftRole::Worker]
}

impl Default for Config {
  fn default() -> Self {
    let name = default_name();
    let data_dir = default_data_dir(&name);
    Self {
      name,
      addr: default_addr(),
      data_dir,
      roles: default_roles(),
      tags: Vec::new(),
      resources: None,
      join_addresses: Vec::new(),
      timeouts: TimeoutsConfig::default(),
    }
  }
}

/// Computes the root directory housing all local Crofts on this host:
/// `~/.swini/crofts`.
pub fn crofts_dir() -> PathBuf {
  let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
  PathBuf::from(home).join(".swini").join("crofts")
}

/// Computes the default root data directory for a named Croft:
/// `~/.swini/crofts/{name}`.
pub fn default_data_dir(name: &str) -> PathBuf {
  crofts_dir().join(name)
}

impl Config {
  /// Loads and resolves configuration according to the hierarchy:
  /// Defaults -> YAML file (`path` or `./swini.yml`) -> Environment variables
  /// (prefixed with `SWINI_`).
  ///
  /// # Errors
  /// Returns an error if the specified config file does not exist or fails to
  /// parse as valid configuration.
  pub fn load(path: Option<&Path>) -> Result<Self, Box<dyn Error>> {
    let mut figment =
      Figment::new().merge(Serialized::defaults(Self::default()));

    let file_path = if let Some(p) = path {
      if !p.exists() {
        return Err(format!("Config file not found: {}", p.display()).into());
      }
      Some(p.to_path_buf())
    } else if Path::new("swini.yml").exists() {
      Some(PathBuf::from("swini.yml"))
    } else {
      None
    };

    let mut has_explicit_data_dir = false;

    if let Some(ref p) = file_path {
      if let Ok(content) = std::fs::read_to_string(p) {
        if let Ok(serde_yml::Value::Mapping(ref m)) =
          serde_yml::from_str::<serde_yml::Value>(&content)
        {
          has_explicit_data_dir =
            m.contains_key(serde_yml::Value::String("data_dir".to_string()));
        }
      }
      figment = figment.merge(Yaml::file(p));
    }

    if std::env::var("SWINI_DATA_DIR").is_ok() {
      has_explicit_data_dir = true;
    }

    let env_provider = Env::prefixed("SWINI_").map(|key| {
      let lower = key.as_str().to_lowercase();
      if let Some(rest) = lower.strip_prefix("timeouts_") {
        format!("timeouts.{}", rest.trim_start_matches('_')).into()
      } else if let Some(rest) = lower.strip_prefix("resources_") {
        format!("resources.{}", rest.trim_start_matches('_')).into()
      } else {
        lower.into()
      }
    });

    figment = figment.merge(env_provider);

    let mut config: Config = figment
      .extract()
      .map_err(|e| format!("Configuration error: {e}"))?;

    if !has_explicit_data_dir {
      config.data_dir = default_data_dir(&config.name);
    }

    Ok(config)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn default_config_values() {
    let _lock = crate::core::TEST_ENV_MUTEX.lock().unwrap();
    let config = Config::default();
    assert_eq!(config.name, "main");
    assert_eq!(config.addr.to_string(), "127.0.0.1:7440");
    assert_eq!(config.roles, vec![CroftRole::Server, CroftRole::Worker]);
    assert!(config.tags.is_empty());
    assert!(config.join_addresses.is_empty());
    assert!(config.data_dir.ends_with(".swini/crofts/main"));
    assert_eq!(config.timeouts.connect, Duration::from_secs(5));
    assert_eq!(config.timeouts.request, Duration::from_secs(15));
    assert_eq!(config.timeouts.join, Duration::from_secs(3));
    assert_eq!(config.timeouts.raft, Duration::from_secs(2));
  }

  #[test]
  fn load_from_custom_yaml_file() {
    let _lock = crate::core::TEST_ENV_MUTEX.lock().unwrap();
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("custom.yml");

    let yaml_content = r#"
name: worker-node
addr: "127.0.0.1:8080"
roles:
  - worker
tags:
  - gpu
  - fast-storage
join_addresses:
  - "127.0.0.1:7440"
timeouts:
  connect: "500ms"
  request: "30s"
"#;
    std::fs::write(&file_path, yaml_content).unwrap();

    let config = Config::load(Some(&file_path)).unwrap();
    assert_eq!(config.name, "worker-node");
    assert_eq!(config.addr.to_string(), "127.0.0.1:8080");
    assert_eq!(config.roles, vec![CroftRole::Worker]);
    assert_eq!(config.tags, vec!["gpu", "fast-storage"]);
    assert_eq!(config.join_addresses, vec!["127.0.0.1:7440"]);
    assert!(config.data_dir.ends_with(".swini/crofts/worker-node"));
    assert_eq!(config.timeouts.connect, Duration::from_millis(500));
    assert_eq!(config.timeouts.request, Duration::from_secs(30));
    assert_eq!(config.timeouts.join, Duration::from_secs(3));
  }

  #[test]
  fn load_missing_config_returns_error() {
    let _lock = crate::core::TEST_ENV_MUTEX.lock().unwrap();
    let path = Path::new("/nonexistent/swini_test_config.yml");
    let result = Config::load(Some(path));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Config file not found"));
  }

  #[test]
  fn env_var_swini_addr_and_timeouts_override() {
    let _lock = crate::core::TEST_ENV_MUTEX.lock().unwrap();
    std::env::set_var("SWINI_ADDR", "127.0.0.1:9999");
    std::env::set_var("SWINI_TIMEOUTS_CONNECT", "250ms");
    std::env::set_var("SWINI_TIMEOUTS_REQUEST", "45s");

    let config = Config::load(None).unwrap();
    assert_eq!(config.addr.to_string(), "127.0.0.1:9999");
    assert_eq!(config.timeouts.connect, Duration::from_millis(250));
    assert_eq!(config.timeouts.request, Duration::from_secs(45));

    std::env::remove_var("SWINI_ADDR");
    std::env::remove_var("SWINI_TIMEOUTS_CONNECT");
    std::env::remove_var("SWINI_TIMEOUTS_REQUEST");
  }
}
