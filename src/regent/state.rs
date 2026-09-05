//! Runtime process state tracking and persistence for Swini Regent instances.
//!
//! Exposes [`RegentState`], which serializes active process metadata (`pid`,
//! `detached`, `started_at`, and the active [`Config`] snapshot) into
//! `{data_dir}/regent.json`.

use crate::croft::Config;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::{Path, PathBuf};

/// Persistent runtime state recorded by a running Regent supervisor process.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegentState {
  /// Operating system process ID of the running Regent.
  pub pid: u32,
  /// Whether the process was started as a detached background daemon.
  pub detached: bool,
  /// ISO-8601 UTC timestamp when the process started.
  pub started_at: String,
  /// Active configuration snapshot used during process execution.
  pub config: Config,
}

impl RegentState {
  /// Computes the filesystem path to the `regent.json` file for a given data
  /// directory.
  pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join("regent.json")
  }

  /// Writes this state snapshot to `{config.data_dir}/regent.json`.
  ///
  /// # Errors
  /// Returns an error if directory creation or file writing fails.
  pub fn save(&self) -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all(&self.config.data_dir)?;
    let state_path = Self::path(&self.config.data_dir);
    let json = serde_json::to_string_pretty(self)?;
    std::fs::write(state_path, json)?;
    Ok(())
  }

  /// Loads and deserializes a [`RegentState`] snapshot from the specified data
  /// directory.
  ///
  /// # Errors
  /// Returns an error if the state file does not exist or fails to parse as
  /// JSON.
  pub fn load(data_dir: &Path) -> Result<Self, Box<dyn Error>> {
    let state_path = Self::path(data_dir);
    let content = std::fs::read_to_string(state_path)?;
    let state = serde_json::from_str(&content)?;
    Ok(state)
  }

  /// Removes the `regent.json` file from disk if present.
  pub fn cleanup(data_dir: &Path) {
    let state_path = Self::path(data_dir);
    let _ = std::fs::remove_file(state_path);
  }

  /// Checks if the operating system process with [`RegentState::pid`] is still
  /// alive.
  pub fn is_alive(&self) -> bool {
    unsafe { libc::kill(self.pid as i32, 0) == 0 }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn state_save_load_cleanup_roundtrip() {
    let dir = tempdir().unwrap();
    let config = Config {
      name: "node-test".to_string(),
      addr: "127.0.0.1:7440".parse().unwrap(),
      data_dir: dir.path().to_path_buf(),
      ..Default::default()
    };

    let state = RegentState {
      pid: std::process::id(),
      detached: false,
      started_at: "2026-09-03T15:00:00Z".to_string(),
      config: config.clone(),
    };

    assert!(state.is_alive());

    state.save().unwrap();
    let loaded = RegentState::load(dir.path()).unwrap();
    assert_eq!(loaded, state);

    RegentState::cleanup(dir.path());
    assert!(RegentState::load(dir.path()).is_err());
  }

  #[test]
  fn dead_pid_is_not_alive() {
    let state = RegentState {
      pid: 4_194_300, // Non-existent PID
      detached: true,
      started_at: String::new(),
      config: Config::default(),
    };

    assert!(!state.is_alive());
  }
}
