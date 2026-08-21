#[derive(Debug, Clone)]
pub struct Config {
  pub heartbeat_interval: u64,
  pub election_timeout_min: u64,
  pub election_timeout_max: u64,
  pub join_addresses: Vec<String>,
}
