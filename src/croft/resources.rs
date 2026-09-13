//! Resource accounting, hardware capacity calculations, and telemetry probing
//! for Crofts.
//!
//! Exposes [`CroftResources`] for representing persistent hardware capacities
//! and allocations, and [`CroftTelemetry`] for capturing live host runtime
//! metrics.

use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Accounting of hardware capacity and active allocations for a Croft in raw
/// units (Hertz and bytes).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CroftResources {
  /// Total physical CPU capacity in Hertz (e.g. 8 cores * 3.2 GHz =
  /// 25_600_000_000 Hz).
  pub cpu_total: u64,
  /// CPU capacity available for scheduling workloads (in Hertz).
  pub cpu_yardable: u64,
  /// CPU capacity committed to active workloads on this Croft (in Hertz,
  /// initialized to 0).
  pub cpu_reserved: u64,
  /// Total physical RAM capacity in bytes.
  pub mem_total: u64,
  /// RAM capacity available for scheduling workloads in bytes.
  pub mem_yardable: u64,
  /// RAM committed to active workloads on this Croft (in bytes, initialized to
  /// 0).
  pub mem_reserved: u64,
}

impl CroftResources {
  /// Returns remaining schedulable CPU capacity in Hertz.
  pub fn cpu_available(&self) -> u64 {
    self.cpu_yardable.saturating_sub(self.cpu_reserved)
  }

  /// Returns remaining schedulable RAM capacity in bytes.
  pub fn mem_available(&self) -> u64 {
    self.mem_yardable.saturating_sub(self.mem_reserved)
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
    let cpu_count = cpus.len() as u64;
    let sum: u64 = cpus.iter().map(|c| c.frequency()).sum();
    let avg_freq_mhz = sum
      .checked_div(cpu_count)
      .filter(|&v| v > 0)
      .unwrap_or(2500);
    let cpu_total = cpu_count.max(1) * avg_freq_mhz * 1_000_000;
    let cpu_yardable = (cpu_total as f64 * 0.90) as u64;

    let mem_total = sys.total_memory();
    let mem_yardable = (mem_total as f64 * 0.90) as u64;

    Self {
      cpu_total,
      cpu_yardable,
      cpu_reserved: 0,
      mem_total,
      mem_yardable,
      mem_reserved: 0,
    }
  }
}

/// Instantaneous live host telemetry snapshot.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct CroftTelemetry {
  /// Current physical memory consumption in bytes.
  pub mem_used: u64,
  /// Current global CPU utilization percentage (0.0 to 100.0).
  pub cpu_used: f32,
}

impl CroftTelemetry {
  /// Samples instantaneous live CPU and RAM consumption from the local host.
  pub fn sample() -> Self {
    let mut sys = System::new_all();
    sys.refresh_all();
    let cpu_used = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    Self { mem_used, cpu_used }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn croft_resources_available_calculation() {
    let resources = CroftResources {
      cpu_total: 10_000_000_000,
      cpu_yardable: 9_000_000_000,
      cpu_reserved: 3_000_000_000,
      mem_total: 16_000_000_000,
      mem_yardable: 14_000_000_000,
      mem_reserved: 4_000_000_000,
    };

    assert_eq!(resources.cpu_available(), 6_000_000_000);
    assert_eq!(resources.mem_available(), 10_000_000_000);

    let oversubscribed = CroftResources {
      cpu_total: 10_000_000_000,
      cpu_yardable: 9_000_000_000,
      cpu_reserved: 12_000_000_000,
      mem_total: 16_000_000_000,
      mem_yardable: 14_000_000_000,
      mem_reserved: 20_000_000_000,
    };
    assert_eq!(oversubscribed.cpu_available(), 0);
    assert_eq!(oversubscribed.mem_available(), 0);
  }

  #[test]
  fn croft_resources_serialization_roundtrip() {
    let resources = CroftResources {
      cpu_total: 25_600_000_000,
      cpu_yardable: 23_040_000_000,
      cpu_reserved: 0,
      mem_total: 34_359_738_368,
      mem_yardable: 30_923_764_531,
      mem_reserved: 0,
    };

    let json = serde_json::to_string(&resources).unwrap();
    let deserialized: CroftResources = serde_json::from_str(&json).unwrap();
    assert_eq!(resources, deserialized);
  }

  #[test]
  fn croft_telemetry_serialization_roundtrip() {
    let telemetry = CroftTelemetry {
      mem_used: 8_589_934_592,
      cpu_used: 15.5,
    };

    let json = serde_json::to_string(&telemetry).unwrap();
    let deserialized: CroftTelemetry = serde_json::from_str(&json).unwrap();
    assert_eq!(telemetry, deserialized);
  }

  #[test]
  fn probe_returns_valid_host_capacities() {
    let resources = CroftResources::probe();
    assert!(resources.cpu_total > 0);
    assert!(resources.cpu_yardable > 0);
    assert_eq!(resources.cpu_reserved, 0);
    assert!(resources.mem_total > 0);
    assert!(resources.mem_yardable > 0);
    assert_eq!(resources.mem_reserved, 0);
  }

  #[test]
  fn sample_telemetry_returns_metrics() {
    let telemetry = CroftTelemetry::sample();
    assert!(telemetry.mem_used > 0);
    assert!(telemetry.cpu_used >= 0.0);
  }
}
