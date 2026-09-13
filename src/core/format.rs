//! Universal human-readable formatting utilities for metric units, frequencies,
//! and byte sizes.
//!
//! Provides scaling formatters [`format_hertz`] and [`format_bytes`] used across
//! CLI command presentations and diagnostic logs.

/// Formats a frequency in Hertz into a human-readable unit string (e.g. `Hz`,
/// `kHz`, `MHz`, `GHz`, `THz`, `PHz`).
///
/// Iteratively scales the value using base-1000 divisors up to petahertz (`PHz`).
pub fn format_hertz(hz: u64) -> String {
  const UNITS: &[&str] = &["Hz", "kHz", "MHz", "GHz", "THz", "PHz"];
  let mut val = hz as f64;
  let mut idx = 0;
  while val >= 1000.0 && idx + 1 < UNITS.len() {
    val /= 1000.0;
    idx += 1;
  }
  if idx == 0 {
    format!("{:.0} {}", val, UNITS[idx])
  } else {
    format!("{:.2} {}", val, UNITS[idx])
  }
}

/// Formats a byte count into a human-readable binary unit string (e.g. `B`,
/// `KiB`, `MiB`, `GiB`, `TiB`, `PiB`).
///
/// Iteratively scales the value using base-1024 divisors up to pebibytes
/// (`PiB`).
pub fn format_bytes(bytes: u64) -> String {
  const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
  let mut val = bytes as f64;
  let mut idx = 0;
  while val >= 1024.0 && idx + 1 < UNITS.len() {
    val /= 1024.0;
    idx += 1;
  }
  if idx == 0 {
    format!("{:.0} {}", val, UNITS[idx])
  } else {
    format!("{:.2} {}", val, UNITS[idx])
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn format_hertz_scales_properly() {
    assert_eq!(format_hertz(0), "0 Hz");
    assert_eq!(format_hertz(500), "500 Hz");
    assert_eq!(format_hertz(1_000), "1.00 kHz");
    assert_eq!(format_hertz(2_400_000), "2.40 MHz");
    assert_eq!(format_hertz(3_200_000_000), "3.20 GHz");
    assert_eq!(format_hertz(16_000_000_000), "16.00 GHz");
    assert_eq!(format_hertz(5_000_000_000_000), "5.00 THz");
    assert_eq!(format_hertz(2_500_000_000_000_000), "2.50 PHz");
  }

  #[test]
  fn format_bytes_scales_properly() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(512), "512 B");
    assert_eq!(format_bytes(1024), "1.00 KiB");
    assert_eq!(format_bytes(1024 * 1024 * 4), "4.00 MiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 16), "16.00 GiB");
    assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024 * 2), "2.00 TiB");
    assert_eq!(
      format_bytes(1024 * 1024 * 1024 * 1024 * 1024 * 3),
      "3.00 PiB"
    );
  }
}
