//! Universal human-readable formatting utilities for metric units, frequencies,
//! and byte sizes, alongside robust case-insensitive parsers for CPU and memory
//! resource strings.
//!
//! Provides scaling formatters [`format_cpu`] and [`format_memory`],
//! string-to-float parsers [`parse_cpu_mhz`] and [`parse_memory_mb`], and serde
//! deserializers for seamless YAML and JSON decoding into canonical float
//! values in MHz (CPU) and MB (Memory).

/// Formats a CPU capacity in megahertz (MHz) into a human-readable SI unit
/// string (`Hz`, `kHz`, `MHz`, `GHz`, `THz`, `PHz`, `EHz`).
pub fn format_cpu(mhz: f64) -> String {
  const UNITS: &[&str] = &["Hz", "kHz", "MHz", "GHz", "THz", "PHz", "EHz"];
  let mut val = mhz * 1_000_000.0;
  let mut idx = 0;
  while val >= 1000.0 && idx + 1 < UNITS.len() {
    val /= 1000.0;
    idx += 1;
  }
  if val.fract() == 0.0 {
    format!("{:.0} {}", val, UNITS[idx])
  } else {
    format!("{:.2} {}", val, UNITS[idx])
  }
}

/// Formats a memory capacity in megabytes (MB) into a human-readable decimal
/// unit string (`B`, `KB`, `MB`, `GB`, `TB`, `PB`, `EB`).
pub fn format_memory(mb: f64) -> String {
  const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  let mut val = mb * 1_000_000.0;
  let mut idx = 0;
  while val >= 1000.0 && idx + 1 < UNITS.len() {
    val /= 1000.0;
    idx += 1;
  }
  if val.fract() == 0.0 {
    format!("{:.0} {}", val, UNITS[idx])
  } else {
    format!("{:.2} {}", val, UNITS[idx])
  }
}

/// Formats a frequency in Hertz into a human-readable unit string (e.g. `Hz`,
/// `kHz`, `MHz`, `GHz`, `THz`, `PHz`).
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

/// Splits a string like `"2.5 GHz"`, `"800mhz"`, `"16"`, or `"512 mb"` into its
/// numeric prefix and unit suffix.
fn split_num_and_unit(s: &str) -> Result<(f64, &str), String> {
  let s = s.trim();
  if s.is_empty() {
    return Err("Resource string cannot be empty".to_string());
  }

  let split_idx = s
    .find(|c: char| !c.is_numeric() && c != '.' && c != '+' && c != '-')
    .unwrap_or(s.len());
  let num_str = s[..split_idx].trim();
  let unit_str = s[split_idx..].trim();

  if num_str.is_empty() {
    return Err(format!("Missing numeric value in '{}'", s));
  }

  let num: f64 = num_str
    .parse()
    .map_err(|e| format!("Invalid number '{}': {}", num_str, e))?;

  Ok((num, unit_str))
}

/// Parses a CPU resource string (case-insensitive) into megahertz (MHz) as
/// `f64`.
///
/// Supported units:
/// - `ghz`, `g` -> `num * 1000.0`
/// - `mhz`, `m`, or no unit -> `num` (defaults to MHz)
/// - `khz`, `k` -> `num / 1000.0`
/// - `hz` -> `num / 1_000_000.0`
/// - `thz`, `t` -> `num * 1_000_000.0`
///
/// # Errors
/// Returns an error if the string is empty, non-numeric, or contains an unknown
/// unit.
pub fn parse_cpu_mhz(input: &str) -> Result<f64, String> {
  let (num, unit) = split_num_and_unit(input)?;
  let u = unit.to_ascii_lowercase();
  match u.as_str() {
    "" | "mhz" | "m" => Ok(num),
    "ghz" | "g" => Ok(num * 1000.0),
    "khz" | "k" => Ok(num / 1000.0),
    "hz" => Ok(num / 1_000_000.0),
    "thz" | "t" => Ok(num * 1_000_000.0),
    other => Err(format!(
      "Unrecognized CPU unit '{}' in '{}'. Expected Hz, kHz, MHz, or GHz.",
      other, input
    )),
  }
}

/// Parses a memory resource string (case-insensitive) into megabytes (MB) as
/// `f64`.
///
/// Supported units:
/// - `tb`, `tib`, `t` -> `num * 1_000_000.0`
/// - `gb`, `gib`, `g` -> `num * 1000.0`
/// - `mb`, `mib`, `m`, or no unit -> `num` (defaults to MB)
/// - `kb`, `kib`, `k` -> `num / 1000.0`
/// - `b`, `bytes` -> `num / 1_000_000.0`
///
/// # Errors
/// Returns an error if the string is empty, non-numeric, or contains an unknown
/// unit.
pub fn parse_memory_mb(input: &str) -> Result<f64, String> {
  let (num, unit) = split_num_and_unit(input)?;
  let u = unit.to_ascii_lowercase();
  match u.as_str() {
    "" | "mb" | "mib" | "m" => Ok(num),
    "gb" | "gib" | "g" => Ok(num * 1000.0),
    "tb" | "tib" | "t" => Ok(num * 1_000_000.0),
    "kb" | "kib" | "k" => Ok(num / 1000.0),
    "b" | "bytes" => Ok(num / 1_000_000.0),
    other => Err(format!(
      "Unrecognized memory unit '{}' in '{}'. Expected B, KB, MB, GB, or TB.",
      other, input
    )),
  }
}

/// Serde deserializer helper decoding a float, integer, or unit string into
/// `f64` MHz.
pub fn deserialize_cpu_mhz<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
  D: serde::Deserializer<'de>,
{
  struct CpuVisitor;

  impl<'de> serde::de::Visitor<'de> for CpuVisitor {
    type Value = f64;

    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      formatter.write_str(
        "a number (in MHz) or CPU string like '500 MHz' or '2.5 GHz'",
      )
    }

    fn visit_i64<E>(self, v: i64) -> Result<f64, E> {
      Ok(v as f64)
    }

    fn visit_u64<E>(self, v: u64) -> Result<f64, E> {
      Ok(v as f64)
    }

    fn visit_f64<E>(self, v: f64) -> Result<f64, E> {
      Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<f64, E>
    where
      E: serde::de::Error,
    {
      parse_cpu_mhz(v).map_err(serde::de::Error::custom)
    }
  }

  deserializer.deserialize_any(CpuVisitor)
}

/// Serde deserializer helper decoding an optional float, integer, or unit
/// string into `Option<f64>` MHz.
pub fn deserialize_opt_cpu_mhz<'de, D>(
  deserializer: D,
) -> Result<Option<f64>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  struct OptCpuVisitor;

  impl<'de> serde::de::Visitor<'de> for OptCpuVisitor {
    type Value = Option<f64>;

    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      formatter.write_str("an optional number or CPU unit string")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
      Ok(None)
    }

    fn visit_some<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
    where
      D2: serde::Deserializer<'de>,
    {
      deserialize_cpu_mhz(deserializer).map(Some)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
      Ok(Some(v as f64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
      Ok(Some(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
      Ok(Some(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
      E: serde::de::Error,
    {
      parse_cpu_mhz(v).map(Some).map_err(serde::de::Error::custom)
    }
  }

  deserializer.deserialize_option(OptCpuVisitor)
}

/// Serde deserializer helper decoding a float, integer, or unit string into
/// `f64` MB.
pub fn deserialize_memory_mb<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
  D: serde::Deserializer<'de>,
{
  struct MemoryVisitor;

  impl<'de> serde::de::Visitor<'de> for MemoryVisitor {
    type Value = f64;

    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      formatter
        .write_str("a number (in MB) or memory string like '512 MB' or '16 GB'")
    }

    fn visit_i64<E>(self, v: i64) -> Result<f64, E> {
      Ok(v as f64)
    }

    fn visit_u64<E>(self, v: u64) -> Result<f64, E> {
      Ok(v as f64)
    }

    fn visit_f64<E>(self, v: f64) -> Result<f64, E> {
      Ok(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<f64, E>
    where
      E: serde::de::Error,
    {
      parse_memory_mb(v).map_err(serde::de::Error::custom)
    }
  }

  deserializer.deserialize_any(MemoryVisitor)
}

/// Serde deserializer helper decoding an optional float, integer, or unit
/// string into `Option<f64>` MB.
pub fn deserialize_opt_memory_mb<'de, D>(
  deserializer: D,
) -> Result<Option<f64>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  struct OptMemoryVisitor;

  impl<'de> serde::de::Visitor<'de> for OptMemoryVisitor {
    type Value = Option<f64>;

    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      formatter.write_str("an optional number or memory unit string")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
      Ok(None)
    }

    fn visit_some<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
    where
      D2: serde::Deserializer<'de>,
    {
      deserialize_memory_mb(deserializer).map(Some)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
      Ok(Some(v as f64))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
      Ok(Some(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
      Ok(Some(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
      E: serde::de::Error,
    {
      parse_memory_mb(v)
        .map(Some)
        .map_err(serde::de::Error::custom)
    }
  }

  deserializer.deserialize_option(OptMemoryVisitor)
}

/// Parses a duration string case-insensitively into a [`std::time::Duration`].
///
/// Supports unit suffixes (`ns`, `us`, `ms`, `s`, `m`, `h`) with optional
/// whitespace. Plain numeric values default to seconds.
///
/// # Examples
/// ```rust
/// use std::time::Duration;
/// use swini::core::format::parse_duration;
///
/// assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
/// assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
/// assert_eq!(parse_duration("1.5s").unwrap(), Duration::from_millis(1500));
/// assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
/// assert_eq!(parse_duration("5").unwrap(), Duration::from_secs(5));
/// ```
pub fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
  let trimmed = s.trim();
  if trimmed.is_empty() {
    return Err("Duration string cannot be empty".to_string());
  }

  // Plain float / integer defaulting to seconds
  if let Ok(secs) = trimmed.parse::<f64>() {
    if secs < 0.0 {
      return Err("Duration cannot be negative".to_string());
    }
    return Ok(std::time::Duration::from_secs_f64(secs));
  }

  let lower = trimmed.to_lowercase();
  let (num_part, unit) =
    if let Some(idx) = lower.find(|c: char| c.is_alphabetic()) {
      let (n, u) = lower.split_at(idx);
      (n.trim(), u.trim())
    } else {
      return Err(format!("Invalid duration format: '{trimmed}'"));
    };

  let val: f64 = num_part
    .parse()
    .map_err(|_| format!("Invalid numeric value in duration: '{num_part}'"))?;

  if val < 0.0 {
    return Err("Duration cannot be negative".to_string());
  }

  match unit {
    "ns" | "nanos" | "nanosecond" | "nanoseconds" => {
      Ok(std::time::Duration::from_nanos(val as u64))
    }
    "us" | "µs" | "micros" | "microsecond" | "microseconds" => {
      Ok(std::time::Duration::from_micros(val as u64))
    }
    "ms" | "millis" | "millisecond" | "milliseconds" => {
      Ok(std::time::Duration::from_secs_f64(val / 1000.0))
    }
    "s" | "sec" | "secs" | "second" | "seconds" => {
      Ok(std::time::Duration::from_secs_f64(val))
    }
    "m" | "min" | "mins" | "minute" | "minutes" => {
      Ok(std::time::Duration::from_secs_f64(val * 60.0))
    }
    "h" | "hr" | "hrs" | "hour" | "hours" => {
      Ok(std::time::Duration::from_secs_f64(val * 3600.0))
    }
    _ => Err(format!("Unknown duration unit: '{unit}' in '{trimmed}'")),
  }
}

/// Serde serializer helper encoding a [`std::time::Duration`] into a
/// millisecond string.
pub fn serialize_duration<S>(
  duration: &std::time::Duration,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  serializer.serialize_str(&format!("{}ms", duration.as_millis()))
}

/// Serde serializer helper encoding an optional [`std::time::Duration`] into a
/// millisecond string.
pub fn serialize_opt_duration<S>(
  duration: &Option<std::time::Duration>,
  serializer: S,
) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  match duration {
    Some(d) => serialize_duration(d, serializer),
    None => serializer.serialize_none(),
  }
}

struct DurationVisitor;

impl<'de> serde::de::Visitor<'de> for DurationVisitor {
  type Value = std::time::Duration;

  fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    formatter.write_str(
      "a number (in seconds), duration string like '500ms', '5s', or a map { secs, nanos }",
    )
  }

  fn visit_i64<E>(self, v: i64) -> Result<std::time::Duration, E>
  where
    E: serde::de::Error,
  {
    if v < 0 {
      return Err(serde::de::Error::custom("Duration cannot be negative"));
    }
    Ok(std::time::Duration::from_secs(v as u64))
  }

  fn visit_u64<E>(self, v: u64) -> Result<std::time::Duration, E> {
    Ok(std::time::Duration::from_secs(v))
  }

  fn visit_f64<E>(self, v: f64) -> Result<std::time::Duration, E>
  where
    E: serde::de::Error,
  {
    if v < 0.0 {
      return Err(serde::de::Error::custom("Duration cannot be negative"));
    }
    Ok(std::time::Duration::from_secs_f64(v))
  }

  fn visit_str<E>(self, v: &str) -> Result<std::time::Duration, E>
  where
    E: serde::de::Error,
  {
    parse_duration(v).map_err(serde::de::Error::custom)
  }

  fn visit_map<M>(self, mut access: M) -> Result<std::time::Duration, M::Error>
  where
    M: serde::de::MapAccess<'de>,
  {
    let mut secs = None;
    let mut nanos = None;
    while let Some((key, value)) =
      access.next_entry::<String, serde_json::Value>()?
    {
      match key.as_str() {
        "secs" => {
          if let Some(s) = value.as_u64() {
            secs = Some(s);
          }
        }
        "nanos" => {
          if let Some(n) = value.as_u64() {
            nanos = Some(n as u32);
          }
        }
        _ => {}
      }
    }
    if let Some(s) = secs {
      Ok(std::time::Duration::new(s, nanos.unwrap_or(0)))
    } else {
      Err(serde::de::Error::custom(
        "Expected map with 'secs' field for Duration",
      ))
    }
  }
}

/// Serde deserializer helper decoding a numeric value (seconds/ms), duration
/// string, or `{ secs, nanos }` map into [`std::time::Duration`].
pub fn deserialize_duration<'de, D>(
  deserializer: D,
) -> Result<std::time::Duration, D::Error>
where
  D: serde::Deserializer<'de>,
{
  deserializer.deserialize_any(DurationVisitor)
}

/// Serde deserializer helper decoding an optional numeric value or duration
/// string into `Option<std::time::Duration>`.
pub fn deserialize_opt_duration<'de, D>(
  deserializer: D,
) -> Result<Option<std::time::Duration>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  struct OptDurationVisitor;

  impl<'de> serde::de::Visitor<'de> for OptDurationVisitor {
    type Value = Option<std::time::Duration>;

    fn expecting(
      &self,
      formatter: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
      formatter.write_str("an optional number or duration unit string")
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
      Ok(None)
    }

    fn visit_some<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
    where
      D2: serde::Deserializer<'de>,
    {
      deserialize_duration(deserializer).map(Some)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
      E: serde::de::Error,
    {
      if v < 0 {
        return Err(serde::de::Error::custom("Duration cannot be negative"));
      }
      Ok(Some(std::time::Duration::from_secs(v as u64)))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
      Ok(Some(std::time::Duration::from_secs(v)))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
      E: serde::de::Error,
    {
      if v < 0.0 {
        return Err(serde::de::Error::custom("Duration cannot be negative"));
      }
      Ok(Some(std::time::Duration::from_secs_f64(v)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
      E: serde::de::Error,
    {
      parse_duration(v)
        .map(Some)
        .map_err(serde::de::Error::custom)
    }

    fn visit_map<M>(
      self,
      access: M,
    ) -> Result<Option<std::time::Duration>, M::Error>
    where
      M: serde::de::MapAccess<'de>,
    {
      DurationVisitor.visit_map(access).map(Some)
    }
  }

  deserializer.deserialize_option(OptDurationVisitor)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_cpu_mhz_units() {
    assert_eq!(parse_cpu_mhz("500").unwrap(), 500.0);
    assert_eq!(parse_cpu_mhz("500mhz").unwrap(), 500.0);
    assert_eq!(parse_cpu_mhz("500 MHz").unwrap(), 500.0);
    assert_eq!(parse_cpu_mhz("500 M").unwrap(), 500.0);
    assert_eq!(parse_cpu_mhz("2.5 ghz").unwrap(), 2500.0);
    assert_eq!(parse_cpu_mhz("2.5GHz").unwrap(), 2500.0);
    assert_eq!(parse_cpu_mhz("2.5 G").unwrap(), 2500.0);
    assert_eq!(parse_cpu_mhz("500000 khz").unwrap(), 500.0);
    assert_eq!(parse_cpu_mhz("2000000000 hz").unwrap(), 2000.0);
    assert!(parse_cpu_mhz("invalid").is_err());
  }

  #[test]
  fn test_parse_memory_mb_units() {
    assert_eq!(parse_memory_mb("512").unwrap(), 512.0);
    assert_eq!(parse_memory_mb("512mb").unwrap(), 512.0);
    assert_eq!(parse_memory_mb("512 mb").unwrap(), 512.0);
    assert_eq!(parse_memory_mb("512 MB").unwrap(), 512.0);
    assert_eq!(parse_memory_mb("512 mib").unwrap(), 512.0);
    assert_eq!(parse_memory_mb("512 m").unwrap(), 512.0);
    assert_eq!(parse_memory_mb("16 gb").unwrap(), 16000.0);
    assert_eq!(parse_memory_mb("16GB").unwrap(), 16000.0);
    assert_eq!(parse_memory_mb("16 g").unwrap(), 16000.0);
    assert_eq!(parse_memory_mb("1024000 kb").unwrap(), 1024.0);
    assert_eq!(parse_memory_mb("1 tb").unwrap(), 1_000_000.0);
    assert!(parse_memory_mb("bad").is_err());
  }

  #[test]
  fn test_format_cpu_and_memory() {
    assert_eq!(format_cpu(0.0005), "500 Hz");
    assert_eq!(format_cpu(0.5), "500 kHz");
    assert_eq!(format_cpu(500.0), "500 MHz");
    assert_eq!(format_cpu(2500.0), "2.50 GHz");
    assert_eq!(format_cpu(5_000_000.0), "5 THz");
    assert_eq!(format_cpu(2_500_000_000.0), "2.50 PHz");

    assert_eq!(format_memory(0.0005), "500 B");
    assert_eq!(format_memory(0.5), "500 KB");
    assert_eq!(format_memory(512.0), "512 MB");
    assert_eq!(format_memory(16000.0), "16 GB");
    assert_eq!(format_memory(2500.0), "2.50 GB");
    assert_eq!(format_memory(1_000_000.0), "1 TB");
    assert_eq!(format_memory(3_000_000_000.0), "3 PB");
  }

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

  #[test]
  fn test_parse_duration() {
    assert_eq!(
      parse_duration("500ms").unwrap(),
      std::time::Duration::from_millis(500)
    );
    assert_eq!(
      parse_duration("500 ms").unwrap(),
      std::time::Duration::from_millis(500)
    );
    assert_eq!(
      parse_duration("5s").unwrap(),
      std::time::Duration::from_secs(5)
    );
    assert_eq!(
      parse_duration("5 secs").unwrap(),
      std::time::Duration::from_secs(5)
    );
    assert_eq!(
      parse_duration("1.5s").unwrap(),
      std::time::Duration::from_millis(1500)
    );
    assert_eq!(
      parse_duration("2m").unwrap(),
      std::time::Duration::from_secs(120)
    );
    assert_eq!(
      parse_duration("1h").unwrap(),
      std::time::Duration::from_secs(3600)
    );
    assert_eq!(
      parse_duration("5").unwrap(),
      std::time::Duration::from_secs(5)
    );
    assert!(parse_duration("invalid").is_err());
    assert!(parse_duration("-5s").is_err());
  }

  #[test]
  fn test_deserialize_duration() {
    #[derive(serde::Deserialize, PartialEq, Debug)]
    struct Wrapper {
      #[serde(deserialize_with = "deserialize_duration")]
      dur: std::time::Duration,
      #[serde(default, deserialize_with = "deserialize_opt_duration")]
      opt_dur: Option<std::time::Duration>,
    }

    let json = r#"{"dur": "5s", "opt_dur": "500ms"}"#;
    let res: Wrapper = serde_json::from_str(json).unwrap();
    assert_eq!(
      res,
      Wrapper {
        dur: std::time::Duration::from_secs(5),
        opt_dur: Some(std::time::Duration::from_millis(500)),
      }
    );

    let json_num = r#"{"dur": 10, "opt_dur": null}"#;
    let res_num: Wrapper = serde_json::from_str(json_num).unwrap();
    assert_eq!(
      res_num,
      Wrapper {
        dur: std::time::Duration::from_secs(10),
        opt_dur: None,
      }
    );
  }
}
