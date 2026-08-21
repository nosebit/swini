use serde::{Deserialize, Serialize};

use crate::store::SpreadNode;

pub type NodeId = u64;

/// A raft node can be a learner or a voter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeRole {
  /// Learners (often called Read Replicas or Non-Voting Members) receive all
  /// the data from the leader so they have a full, up-to-date copy of the
  /// database. However, they do not vote, and the leader does not wait for
  /// them to acknowledge data before committing it. They are strictly there to
  /// "listen and learn."
  Learner,

  /// Voters are the core participants. They vote in elections to pick the
  /// leader, and when the leader wants to save ("commit") a new piece of data,
  /// it must wait for a majority of the Voters to acknowledge it. Because the
  /// leader has to wait for them, adding too many Voters can slow down your
  /// write speed.
  Voter,
}

/// This block of code implements the Default trait for the NodeRole
/// enum. It tells Rust, "If someone asks for a default NodeRole without
/// specifying which one, give them Learner."
impl Default for NodeRole {
  fn default() -> Self {
    Self::Learner
  }
}

/// This represents a Raft node which implements the SpreadNode trait.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Node {
  pub id: u64,
  pub name: String,
  pub api_addr: String,
  pub role: NodeRole,
  pub is_leader: bool,
}

impl SpreadNode for Node {
  fn new(id: u64, name: String, api_addr: String) -> Self {
    Self {
      id,
      name,
      api_addr,
      role: NodeRole::Voter,
      is_leader: false,
    }
  }

  fn id(&self) -> u64 {
    self.id
  }

  fn name(&self) -> String {
    self.name.clone()
  }

  fn api_addr(&self) -> String {
    self.api_addr.clone()
  }
}
