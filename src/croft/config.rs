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

use crate::croft::types::CroftRole;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

/// Default socket address where the Croft's Gate and Clerk listen for incoming
/// Ranch traffic.
pub const DEFAULT_ADDR: &str = "127.0.0.1:7440";

/// Default instance name for the local Croft and Regent runtime process.
pub const DEFAULT_NAME: &str = "main";

/// Runtime configuration for a Swini Croft and its supervising Regent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

  /// Bootstrap peer addresses to contact when joining an existing Ranch
  /// cluster.
  #[serde(default)]
  pub join_addresses: Vec<String>,
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
      join_addresses: Vec::new(),
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
  /// Defaults -> YAML file (`path` or `./swini.yml`) -> Environment variables.
  ///
  /// # Errors
  /// Returns an error if the specified config file does not exist or fails to
  /// parse as valid YAML.
  pub fn load(path: Option<&Path>) -> Result<Self, Box<dyn Error>> {
    let mut config = Self::default();

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

    if let Some(ref p) = file_path {
      let content = std::fs::read_to_string(p)?;
      let file_value: serde_yml::Value = serde_yml::from_str(&content)?;
      let has_explicit_data_dir = matches!(
        &file_value,
        serde_yml::Value::Mapping(m) if m.contains_key(serde_yml::Value::String("data_dir".to_string()))
      );

      let default_value = serde_yml::to_value(&config)?;
      let merged_value = merge_yaml_values(default_value, file_value);
      config = serde_yml::from_value(merged_value)?;

      if !has_explicit_data_dir {
        config.data_dir = default_data_dir(&config.name);
      }
    }

    if let Ok(data_dir_str) = std::env::var("SWINI_DATA_DIR") {
      let trimmed = data_dir_str.trim();
      if !trimmed.is_empty() {
        config.data_dir = PathBuf::from(trimmed);
      }
    }

    if let Ok(addr_str) = std::env::var("SWINI_ADDR") {
      let trimmed = addr_str.trim();
      if !trimmed.is_empty() {
        if let Ok(parsed) = trimmed.parse::<SocketAddr>() {
          config.addr = parsed;
        }
      }
    }

    Ok(config)
  }
}

/// Recursively merges YAML values where `override_val` takes precedence over
/// `base`.
fn merge_yaml_values(
  base: serde_yml::Value,
  override_val: serde_yml::Value,
) -> serde_yml::Value {
  match (base, override_val) {
    (
      serde_yml::Value::Mapping(mut base_map),
      serde_yml::Value::Mapping(override_map),
    ) => {
      for (k, v) in override_map {
        if let Some(existing) = base_map.remove(&k) {
          base_map.insert(k, merge_yaml_values(existing, v));
        } else {
          base_map.insert(k, v);
        }
      }
      serde_yml::Value::Mapping(base_map)
    }
    (_, override_val) => override_val,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn default_config_values() {
    let config = Config::default();
    assert_eq!(config.name, "main");
    assert_eq!(config.addr.to_string(), "127.0.0.1:7440");
    assert_eq!(config.roles, vec![CroftRole::Server, CroftRole::Worker]);
    assert!(config.tags.is_empty());
    assert!(config.join_addresses.is_empty());
    assert!(config.data_dir.ends_with(".swini/crofts/main"));
  }

  #[test]
  fn load_from_custom_yaml_file() {
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
"#;
    std::fs::write(&file_path, yaml_content).unwrap();

    let config = Config::load(Some(&file_path)).unwrap();
    assert_eq!(config.name, "worker-node");
    assert_eq!(config.addr.to_string(), "127.0.0.1:8080");
    assert_eq!(config.roles, vec![CroftRole::Worker]);
    assert_eq!(config.tags, vec!["gpu", "fast-storage"]);
    assert_eq!(config.join_addresses, vec!["127.0.0.1:7440"]);
    assert!(config.data_dir.ends_with(".swini/crofts/worker-node"));
  }

  #[test]
  fn load_missing_config_returns_error() {
    let path = Path::new("/nonexistent/swini_test_config.yml");
    let result = Config::load(Some(path));
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Config file not found"));
  }

  #[test]
  fn env_var_swini_addr_override() {
    std::env::set_var("SWINI_ADDR", "127.0.0.1:9999");
    let config = Config::load(None).unwrap();
    assert_eq!(config.addr.to_string(), "127.0.0.1:9999");
    std::env::remove_var("SWINI_ADDR");
  }
}
