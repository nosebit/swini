use std::path::PathBuf;

use super::raft;

#[derive(Debug, Clone)]
pub struct Config {
  pub node_id: u64,
  pub api_addr: String,
  pub data_dir: PathBuf,
  pub raft: raft::Config,
}
