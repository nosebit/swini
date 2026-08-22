use super::raft;

#[derive(Debug, Clone)]
pub struct BarnConfig {
  pub raft: raft::Config,
}
