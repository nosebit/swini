//! Workload YAML configuration parsing, schema validation, and normalization.
//!
//! Exposes [`PigRunConfig`], [`HerdConfig`], and [`PigDefaultsConfig`] used to
//! parse declarative `pigs.yml` manifests containing top-level and herd-grouped
//! workload specifications.

use crate::pig::types::{Pig, PigConfig, PigStatus, Placement, DEFAULT_SPACE};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::path::Path;

/// Top-level declarative workload configuration parsed from `pigs.yml`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PigRunConfig {
  /// Default namespace applied to all contained pigs unless overridden.
  #[serde(default = "default_space_val")]
  pub space: String,
  /// Direct top-level list of Pig configurations.
  #[serde(default)]
  pub pigs: Vec<PigConfig>,
  /// Optional herd groupings organizing pigs into logical sets.
  #[serde(default)]
  pub herds: Vec<HerdConfig>,
}

fn default_space_val() -> String {
  DEFAULT_SPACE.to_string()
}

/// Weak organizational grouping of pigs within a configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct HerdConfig {
  /// Name of the herd grouping.
  pub name: String,
  /// Default configuration inherited by pigs within this herd.
  #[serde(default)]
  pub defaults: Option<PigDefaultsConfig>,
  /// List of Pig configurations belonging to this herd.
  #[serde(default)]
  pub pigs: Vec<PigConfig>,
}

/// Default values inherited by pigs in a herd when not explicitly specified on
/// the pig.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PigDefaultsConfig {
  /// Default tags inherited by pigs in this herd.
  #[serde(default)]
  pub tags: Vec<String>,
  /// Optional default replica size inherited by pigs in this herd.
  pub size: Option<u32>,
  /// Optional default placement constraints and Yard allocations.
  pub placement: Option<Placement>,
}

impl PigRunConfig {
  /// Loads and parses a configuration YAML file from disk.
  ///
  /// # Errors
  /// Returns an error if the file cannot be read, contains invalid YAML, or
  /// fails validation.
  pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
    let content = std::fs::read_to_string(path)?;
    Self::from_yaml(&content)
  }

  /// Parses YAML content and normalizes pigs across herds.
  ///
  /// # Errors
  /// Returns an error if the YAML syntax is invalid, has duplicate names, or
  /// missing task fields.
  pub fn from_yaml(yaml_str: &str) -> Result<Self, Box<dyn Error>> {
    let mut config: PigRunConfig = serde_yml::from_str(yaml_str)?;
    config.normalize()?;
    Ok(config)
  }

  /// Flattens herds into `self.pigs`, applies inherited space and defaults,
  /// and validates that pig names are unique across all herds.
  ///
  /// # Errors
  /// Returns an error if duplicate pig names or invalid task structures are
  /// found.
  pub fn normalize(&mut self) -> Result<(), Box<dyn Error>> {
    let mut all_pigs = Vec::new();
    let mut seen_names = HashSet::new();

    for mut pig in self.pigs.drain(..) {
      if pig.space.is_empty() {
        pig.space = if self.space.is_empty() {
          DEFAULT_SPACE.to_string()
        } else {
          self.space.clone()
        };
      }
      Self::pig_validate(&pig)?;
      if !seen_names.insert(pig.name.clone()) {
        return Err(
          format!("Duplicate pig name '{}' in configuration", pig.name).into(),
        );
      }
      all_pigs.push(pig);
    }

    for herd in &self.herds {
      for mut pig in herd.pigs.clone() {
        if pig.space.is_empty() {
          pig.space = if self.space.is_empty() {
            DEFAULT_SPACE.to_string()
          } else {
            self.space.clone()
          };
        }
        pig.herd = Some(herd.name.clone());

        // Apply herd defaults if defined
        if let Some(ref defaults) = herd.defaults {
          for tag in &defaults.tags {
            if !pig.tags.contains(tag) {
              pig.tags.push(tag.clone());
            }
          }
          if let Some(size) = defaults.size {
            if pig.size == 1 {
              pig.size = size;
            }
          }
          if let Some(ref placement) = defaults.placement {
            if pig.piglet.placement == Placement::default() {
              pig.piglet.placement = placement.clone();
            }
          }
        }

        Self::pig_validate(&pig)?;
        if !seen_names.insert(pig.name.clone()) {
          return Err(
            format!(
              "Duplicate pig name '{}' in herd '{}'",
              pig.name, herd.name
            )
            .into(),
          );
        }
        all_pigs.push(pig);
      }
    }

    self.pigs = all_pigs;
    Ok(())
  }

  /// Converts all normalized pig configurations into domain [`Pig`] entities.
  pub fn to_pigs(&self, now: u64) -> Vec<Pig> {
    self
      .pigs
      .iter()
      .map(|pig_cfg| Pig {
        path: format!("{}/{}", pig_cfg.space, pig_cfg.name),
        config: pig_cfg.clone(),
        status: PigStatus::Received,
        replace_count: 0,
        replaced_at: None,
        created_at: now,
      })
      .collect()
  }

  fn pig_validate(pig: &PigConfig) -> Result<(), Box<dyn Error>> {
    if pig.name.trim().is_empty() {
      return Err("Pig name cannot be empty".into());
    }
    if pig.piglet.tasks.is_empty() {
      return Err(
        format!("Pig '{}' must define at least one task", pig.name).into(),
      );
    }
    for task in &pig.piglet.tasks {
      if task.name.trim().is_empty() {
        return Err(
          format!("Task name in pig '{}' cannot be empty", pig.name).into(),
        );
      }
      if task.exec.is_none() {
        return Err(
          format!(
            "Task '{}' in pig '{}' must specify an exec command",
            task.name, pig.name
          )
          .into(),
        );
      }
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::tempdir;

  #[test]
  fn test_parse_simple_pig_config() {
    let yaml = r#"
space: staging
pigs:
  - name: api
    size: 2
    tags: ["web"]
    piglet:
      tasks:
        - name: web
          exec:
            command: ["./server", "--port=8080"]
"#;

    let config = PigRunConfig::from_yaml(yaml).unwrap();
    assert_eq!(config.space, "staging");
    assert_eq!(config.pigs.len(), 1);

    let pig = &config.pigs[0];
    assert_eq!(pig.name, "api");
    assert_eq!(pig.space, "staging");
    assert_eq!(pig.size, 2);
    assert_eq!(pig.tags, vec!["web"]);
    assert_eq!(pig.piglet.tasks.len(), 1);
    assert_eq!(pig.piglet.tasks[0].name, "web");

    let pigs = config.to_pigs(1726500000);
    assert_eq!(pigs.len(), 1);
    assert_eq!(pigs[0].path, "staging/api");
    assert_eq!(pigs[0].status, PigStatus::Received);
  }

  #[test]
  fn test_parse_herd_with_defaults() {
    let yaml = r#"
herds:
  - name: workers
    defaults:
      tags: ["worker-node"]
      size: 3
    pigs:
      - name: email-worker
        piglet:
          tasks:
            - name: process
              exec:
                command: ["./worker", "email"]
      - name: report-worker
        size: 5
        piglet:
          tasks:
            - name: process
              exec:
                command: ["./worker", "report"]
"#;

    let config = PigRunConfig::from_yaml(yaml).unwrap();
    assert_eq!(config.pigs.len(), 2);

    let p1 = &config.pigs[0];
    assert_eq!(p1.name, "email-worker");
    assert_eq!(p1.size, 3); // inherited default
    assert_eq!(p1.tags, vec!["worker-node"]);
    assert_eq!(p1.herd.as_deref(), Some("workers"));

    let p2 = &config.pigs[1];
    assert_eq!(p2.name, "report-worker");
    assert_eq!(p2.size, 5); // explicit override
    assert_eq!(p2.tags, vec!["worker-node"]);
    assert_eq!(p2.herd.as_deref(), Some("workers"));
  }

  #[test]
  fn test_duplicate_pig_name_error() {
    let yaml = r#"
pigs:
  - name: same-name
    piglet:
      tasks:
        - name: t1
          exec:
            command: ["ls"]
  - name: same-name
    piglet:
      tasks:
        - name: t2
          exec:
            command: ["pwd"]
"#;

    let result = PigRunConfig::from_yaml(yaml);
    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Duplicate pig name 'same-name'"));
  }

  #[test]
  fn test_load_from_file() {
    let dir = tempdir().unwrap();
    let file = dir.path().join("pigs.yml");
    std::fs::write(
      &file,
      r#"
pigs:
  - name: simple
    piglet:
      tasks:
        - name: run
          exec:
            command: ["echo", "1"]
"#,
    )
    .unwrap();

    let config = PigRunConfig::load(&file).unwrap();
    assert_eq!(config.pigs.len(), 1);
    assert_eq!(config.pigs[0].name, "simple");
  }
}
