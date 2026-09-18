//! Resource accounting, hardware capacity calculations, and telemetry probing
//! for Crofts.
//!
//! Exposes [`CroftResources`] for representing persistent hardware capacities
//! and allocations, and [`CroftTelemetry`] for capturing live host runtime
//! metrics.

use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Accounting of hardware capacity and active allocations for a Croft in
/// megahertz (MHz) and megabytes (MB).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CroftResources {
  /// Total physical or configured CPU capacity in MHz (e.g. 8 cores * 3200 MHz
  /// = 25600.0).
  pub cpu_total: f64,
  /// CPU capacity available for scheduling workloads (in MHz).
  pub cpu_yardable: f64,
  /// CPU capacity committed to active workloads on this Croft (in MHz,
  /// initialized to 0.0).
  pub cpu_reserved: f64,
  /// Total physical or configured RAM capacity in MB (e.g. 16000.0).
  pub mem_total: f64,
  /// RAM capacity available for scheduling workloads in MB.
  pub mem_yardable: f64,
  /// RAM committed to active workloads on this Croft (in MB, initialized to
  /// 0.0).
  pub mem_reserved: f64,
}

impl CroftResources {
  /// Returns remaining schedulable CPU capacity in MHz.
  pub fn cpu_available(&self) -> f64 {
    (self.cpu_yardable - self.cpu_reserved).max(0.0)
  }

  /// Returns remaining schedulable RAM capacity in MB.
  pub fn mem_available(&self) -> f64 {
    (self.mem_yardable - self.mem_reserved).max(0.0)
  }

  /// Probes local host hardware capacity and returns baseline
  /// [`CroftResources`].
  ///
  /// Evaluates total CPU core count, clock frequencies, and total physical RAM,
  /// allocating 90% as yardable workload capacity by default.
  pub fn probe() -> Self {
    let mut sys = System::new_all();
    sys.refresh_all();

    let cpus = sys.cpus();
    let cpu_count = cpus.len() as f64;
    let sum: u64 = cpus.iter().map(|c| c.frequency()).sum();
    let avg_freq_mhz = if cpu_count > 0.0 {
      let avg = sum as f64 / cpu_count;
      if avg > 0.0 {
        avg
      } else {
        2500.0
      }
    } else {
      2500.0
    };
    let cpu_total = (cpu_count.max(1.0) * avg_freq_mhz * 100.0).round() / 100.0;
    let cpu_yardable = (cpu_total * 0.90 * 100.0).round() / 100.0;

    let mem_bytes = sys.total_memory() as f64;
    let mem_total = (mem_bytes / 1_000_000.0 * 100.0).round() / 100.0;
    let mem_yardable = (mem_total * 0.90 * 100.0).round() / 100.0;

    Self {
      cpu_total,
      cpu_yardable,
      cpu_reserved: 0.0,
      mem_total,
      mem_yardable,
      mem_reserved: 0.0,
    }
  }
}

/// Instantaneous live host telemetry snapshot in MHz and MB.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CroftTelemetry {
  /// Instantaneous physical CPU clock consumption in MHz.
  pub cpu_used: f64,
  /// Current physical memory consumption in MB.
  pub mem_used: f64,
}

impl CroftTelemetry {
  /// Samples instantaneous live CPU and RAM consumption from the local host in
  /// MHz and MB.
  pub fn sample(total_cpu_mhz: f64) -> Self {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_ratio = (sys.global_cpu_usage() as f64 / 100.0).max(0.0);
    let cpu_used = total_cpu_mhz * cpu_ratio;
    let mem_used = sys.used_memory() as f64 / 1_000_000.0;

    Self { cpu_used, mem_used }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn croft_resources_available_calculation() {
    let resources = CroftResources {
      cpu_total: 10000.0,
      cpu_yardable: 9000.0,
      cpu_reserved: 3000.0,
      mem_total: 16000.0,
      mem_yardable: 14000.0,
      mem_reserved: 4000.0,
    };

    assert_eq!(resources.cpu_available(), 6000.0);
    assert_eq!(resources.mem_available(), 10000.0);
  }

  #[test]
  fn croft_resources_default_saturation() {
    let resources = CroftResources {
      cpu_total: 4000.0,
      cpu_yardable: 3500.0,
      cpu_reserved: 4000.0,
      mem_total: 8000.0,
      mem_yardable: 7000.0,
      mem_reserved: 7500.0,
    };

    assert_eq!(resources.cpu_available(), 0.0);
    assert_eq!(resources.mem_available(), 0.0);
  }

  #[test]
  fn croft_resources_serialization_roundtrip() {
    let resources = CroftResources {
      cpu_total: 32000.0,
      cpu_yardable: 28800.0,
      cpu_reserved: 0.0,
      mem_total: 32000.0,
      mem_yardable: 28800.0,
      mem_reserved: 0.0,
    };

    let json = serde_json::to_string(&resources).unwrap();
    let deserialized: CroftResources = serde_json::from_str(&json).unwrap();
    assert_eq!(resources, deserialized);
  }

  #[test]
  fn croft_telemetry_serialization_roundtrip() {
    let telemetry = CroftTelemetry {
      cpu_used: 2480.0,
      mem_used: 8000.0,
    };

    let json = serde_json::to_string(&telemetry).unwrap();
    let deserialized: CroftTelemetry = serde_json::from_str(&json).unwrap();
    assert_eq!(telemetry, deserialized);
  }

  #[test]
  fn probe_returns_valid_host_capacities() {
    let resources = CroftResources::probe();
    assert!(resources.cpu_total > 0.0);
    assert!(resources.cpu_yardable > 0.0);
    assert_eq!(resources.cpu_reserved, 0.0);
    assert!(resources.mem_total > 0.0);
    assert!(resources.mem_yardable > 0.0);
    assert_eq!(resources.mem_reserved, 0.0);
  }

  #[test]
  fn sample_telemetry_returns_metrics() {
    let telemetry = CroftTelemetry::sample(16000.0);
    assert!(telemetry.mem_used > 0.0);
    assert!(telemetry.cpu_used >= 0.0);
  }
}
