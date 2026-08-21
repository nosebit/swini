use serde::{Deserialize, Serialize};
use std::io::Cursor;

use super::raft;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Action {
  Set { key: String, value: Vec<u8> },
  Delete { key: String },
  Patch { key: String, value: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadAction {
  Get { key: String, allow_stale: bool },
  List { prefix: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReadResult {
  GetResult(Option<Vec<u8>>),
  ListResult(Vec<(String, Vec<u8>)>),
  Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ActionResult {
  Success,
  Error(String),
}

openraft::declare_raft_types!(
    pub TypeConfig:
        D = Action,
        R = ActionResult,
        Node = raft::Node,
);
