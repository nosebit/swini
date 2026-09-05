pub mod api;
mod config;
mod events;
mod raft;
mod storage;
mod types;

pub use config::Config;
pub use events::Event;
pub use raft::Node;
pub use types::{Action, ActionResult, ReadAction, ReadResult};

use crate::core::proto::barn::barn::barn_api_client::BarnApiClient;
use crate::core::proto::barn::barn::barn_api_server::BarnApiServer;
use crate::core::proto::barn::barn::BarnMessage;
use crate::store::{
  ClusterStore, ItemStore, ItemStoreEvent, SpreadNode, SpreadStore, Store,
  StoreEvent,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::sync::Arc;
use storage::Storage;
use tokio::sync::{broadcast, RwLock};
use types::TypeConfig;

/// Barn is swini's replicated key/value store: the concrete implementation
/// of `ClusterStore` (in turn `ItemStore` + `SpreadStore`) backed by a
/// raft consensus group. See the module README for the overall design.
#[derive(Clone)]
pub struct Barn {
  self_node: raft::Node,
  raft: openraft::Raft<TypeConfig>,
  storage: Storage,
  events_tx: broadcast::Sender<StoreEvent<Event>>,
  node_cache: Arc<RwLock<Vec<raft::Node>>>,
  /// Non-empty when this handle was returned by `slice`; every key this
  /// handle touches is transparently prefixed with it.
  prefix: String,
}

impl Barn {
  /// Exposes the gRPC [`BarnApiServer`] service instance for this Barn.
  pub fn api(&self) -> BarnApiServer<api::Api> {
    api::Api::new(self.clone()).into_server()
  }

  /// Opens local storage, brings up the raft engine on top of it, and
  /// starts the background task that turns raft/storage activity into
  /// `Event`s. Does not join or bootstrap any cluster membership — that's
  /// `SpreadStore::node_add`'s job, called by whoever is orchestrating
  /// cluster formation.
  pub async fn spawn(
    gate: &crate::croft::Gate,
    config: Config,
  ) -> Result<Self, Box<dyn Error>> {
    std::fs::create_dir_all(&config.data_dir)?;
    let data_db =
      Arc::new(redb::Database::create(config.data_dir.join("barn.data"))?);
    let storage = Storage::new(data_db)?;

    let raft = raft::create::<TypeConfig, Storage>(
      config.id,
      &raft::Config {
        heartbeat_interval: config.heartbeat_interval,
        election_timeout_min: config.election_timeout_min,
        election_timeout_max: config.election_timeout_max,
      },
      config.data_dir.join("barn"),
      storage.clone(),
    )
    .await?;

    let self_node = raft::Node::new(config.id, config.addr);
    let (events_tx, _) = broadcast::channel(1024);
    let node_cache = Arc::new(RwLock::new(Self::nodes_from_metrics(
      &raft.metrics().borrow(),
    )));

    let barn = Self {
      self_node,
      raft,
      storage,
      events_tx: events_tx.clone(),
      node_cache,
      prefix: String::new(),
    };

    gate.add(barn.api());

    barn.event_forwarder_start();
    let _ = events_tx.send(StoreEvent::Initialized);

    Ok(barn)
  }

  fn scoped_key(&self, key: &str) -> String {
    format!("{}{key}", self.prefix)
  }

  /// Derives the current node list from a raft metrics snapshot, with each
  /// `Node`'s `role`/`is_leader` freshly computed (the copy raft itself
  /// stores in membership can be stale — these two fields are the only
  /// ones that actually change after a node is added). `event_forwarder_start`
  /// calls this on every metrics change to refresh `node_cache`, which is
  /// the single thing `get`/`set`/`stale_get`/`node`/`node_list` read from
  /// — never `raft.metrics()` directly.
  fn nodes_from_metrics(
    metrics: &openraft::RaftMetrics<raft::NodeId, raft::Node>,
  ) -> Vec<raft::Node> {
    let voters: HashSet<_> = metrics.membership_config.voter_ids().collect();
    metrics
      .membership_config
      .nodes()
      .map(|(id, node)| {
        let mut node = node.clone();
        node.role = if voters.contains(id) {
          raft::NodeRole::Voter
        } else {
          raft::NodeRole::Learner
        };
        node.is_leader = metrics.current_leader == Some(*id);
        node
      })
      .collect()
  }

  async fn is_leader(&self) -> bool {
    self
      .node_cache
      .read()
      .await
      .iter()
      .any(|node| node.id == self.self_node.id && node.is_leader)
  }

  /// The `Node` entry for the current raft leader, read from `node_cache`
  /// — `None` until a leader has been elected.
  async fn current_leader_node(&self) -> Option<raft::Node> {
    self
      .node_cache
      .read()
      .await
      .iter()
      .find(|node| node.is_leader)
      .cloned()
  }

  /// Accessor for `api::barn::BarnApiHandler`, which needs to call the raft
  /// transport RPCs (`append_entries`/`vote`/`install_snapshot`) directly.
  pub(crate) fn raft(&self) -> &openraft::Raft<TypeConfig> {
    &self.raft
  }

  fn dial(node: &raft::Node) -> BarnApiClient<tonic::transport::Channel> {
    let addr = format!("http://{}", node.api_addr);
    let channel = tonic::transport::Endpoint::from_shared(addr)
      .expect("node api_addr must be a valid URI")
      .connect_lazy();
    BarnApiClient::new(channel)
  }

  /// Answers a `ReadAction` against this node's own storage. `allow_stale:
  /// false` first confirms this node is still actually leader
  /// (`ensure_linearizable`) before reading — callers only reach this path
  /// once `is_leader()` already agreed, but leadership can change in the
  /// gap, so this is the authoritative check.
  pub(crate) async fn local_read(&self, action: ReadAction) -> ReadResult {
    let allow_stale = matches!(
      action,
      ReadAction::Get {
        allow_stale: true,
        ..
      }
    );
    if !allow_stale {
      if let Err(err) = self.raft.ensure_linearizable().await {
        return ReadResult::Error(err.to_string());
      }
    }
    self.storage.read(action)
  }

  /// Proposes `action` to the raft log via this node's own handle. Called
  /// both when this node believes itself leader, and by
  /// `BarnApiHandler::forward_write` when another node forwarded a write
  /// here believing *this* node is the leader.
  pub(crate) async fn local_write(&self, action: Action) -> ActionResult {
    match self.raft.client_write(action).await {
      Ok(response) => response.data,
      Err(err) => ActionResult::Error(err.to_string()),
    }
  }

  async fn forward_read(
    &self,
    node: &raft::Node,
    action: ReadAction,
  ) -> Result<ReadResult, Box<dyn Error>> {
    let payload = serde_json::to_vec(&action)?;
    let response = Self::dial(node)
      .forward_read(BarnMessage { payload })
      .await?;
    Ok(serde_json::from_slice(&response.into_inner().payload)?)
  }

  async fn forward_write(
    &self,
    node: &raft::Node,
    action: Action,
  ) -> Result<ActionResult, Box<dyn Error>> {
    let payload = serde_json::to_vec(&action)?;
    let response = Self::dial(node)
      .forward_write(BarnMessage { payload })
      .await?;
    Ok(serde_json::from_slice(&response.into_inner().payload)?)
  }

  /// Reads (possibly stale) from a randomly picked cluster member,
  /// including possibly this node itself — in which case no network hop is
  /// made at all.
  pub async fn stale_get(
    &self,
    key: &str,
  ) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    let action = ReadAction::Get {
      key: self.scoped_key(key),
      allow_stale: true,
    };
    let target = self
      .random_node()
      .await
      .unwrap_or_else(|| self.self_node.clone());

    let result = if target.id == self.self_node.id {
      self.storage.read(action)
    } else {
      self.forward_read(&target, action).await?
    };

    match result {
      ReadResult::GetResult(value) => Ok(value),
      ReadResult::Error(err) => Err(err.into()),
      _ => unreachable!("Get always returns GetResult or Error"),
    }
  }

  async fn random_node(&self) -> Option<raft::Node> {
    use rand::seq::IndexedRandom;
    let nodes = self.node_cache.read().await;
    nodes.choose(&mut rand::rng()).cloned()
  }

  /// Watches raft's own metrics (membership/leader changes) and storage's
  /// applied-write events. Metrics changes refresh `node_cache` (the
  /// single source of truth `is_leader`/`current_leader_node`/
  /// `random_node`/`node`/`node_list` all read from) and are diffed
  /// against the previous snapshot to emit `NodeAdded`/`NodeRemoved`/
  /// `NodeChanged` `Event`s; storage's applied-write events are forwarded
  /// as `ItemCreated`/`ItemPatched`/`ItemRemoved`. One task per `Barn`,
  /// started once in `spawn`.
  fn event_forwarder_start(&self) {
    let mut metrics_rx = self.raft.metrics();
    let mut item_rx = self.storage.subscribe();
    let events_tx = self.events_tx.clone();
    let node_cache = self.node_cache.clone();

    tokio::spawn(async move {
      let mut last_nodes: HashMap<raft::NodeId, raft::Node> = HashMap::new();

      loop {
        tokio::select! {
          Ok(()) = metrics_rx.changed() => {
            let metrics = metrics_rx.borrow().clone();
            let nodes = Barn::nodes_from_metrics(&metrics);
            let current: HashMap<raft::NodeId, raft::Node> =
              nodes.iter().map(|node| (node.id, node.clone())).collect();
            *node_cache.write().await = nodes;

            for (id, node) in &current {
              let event = match last_nodes.get(id) {
                None => Some(Event::NodeAdded(node.clone())),
                Some(prev) if prev != node => Some(Event::NodeChanged(node.clone())),
                _ => None,
              };
              if let Some(event) = event {
                let _ = events_tx.send(StoreEvent::Custom(event));
              }
            }
            for (id, node) in &last_nodes {
              if !current.contains_key(id) {
                let event = Event::NodeRemoved(node.clone());
                let _ = events_tx.send(StoreEvent::Custom(event));
              }
            }
            last_nodes = current;
          }
          Ok(item_event) = item_rx.recv() => {
            let event = match item_event {
              ItemStoreEvent::ItemCreated(k, v) => Event::ItemCreated(k, v),
              ItemStoreEvent::ItemPatched(k, v) => Event::ItemPatched(k, v),
              ItemStoreEvent::ItemRemoved(k) => Event::ItemRemoved(k),
            };
            let _ = events_tx.send(StoreEvent::Custom(event));
          }
          else => break,
        }
      }
    });
  }
}

#[async_trait::async_trait]
impl Store for Barn {
  type Event = Event;

  async fn subscribe(&self) -> broadcast::Receiver<StoreEvent<Self::Event>> {
    self.events_tx.subscribe()
  }
}

#[async_trait::async_trait]
impl ItemStore for Barn {
  type Key = String;
  type Value = Vec<u8>;

  fn slice(&self, prefix: &str) -> Self {
    Self {
      prefix: self.scoped_key(prefix),
      ..self.clone()
    }
  }

  async fn get(&self, key: &String) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    let action = ReadAction::Get {
      key: self.scoped_key(key),
      allow_stale: false,
    };
    let result = if self.is_leader().await {
      self.local_read(action).await
    } else {
      let leader = self
        .current_leader_node()
        .await
        .ok_or("no known raft leader")?;
      self.forward_read(&leader, action).await?
    };
    match result {
      ReadResult::GetResult(value) => Ok(value),
      ReadResult::Error(err) => Err(err.into()),
      _ => unreachable!("Get always returns GetResult or Error"),
    }
  }

  async fn set(
    &self,
    key: &String,
    value: Vec<u8>,
  ) -> Result<(), Box<dyn Error>> {
    self
      .write(Action::Set {
        key: self.scoped_key(key),
        value,
      })
      .await
  }

  async fn delete(&self, key: &String) -> Result<(), Box<dyn Error>> {
    self
      .write(Action::Delete {
        key: self.scoped_key(key),
      })
      .await
  }

  async fn list(
    &self,
    prefix: Option<&String>,
  ) -> Result<Vec<(String, Vec<u8>)>, Box<dyn Error>> {
    let scoped = self.scoped_key(prefix.map(String::as_str).unwrap_or(""));
    let action = ReadAction::List {
      prefix: Some(scoped),
    };
    let result = if self.is_leader().await {
      self.local_read(action).await
    } else {
      let leader = self
        .current_leader_node()
        .await
        .ok_or("no known raft leader")?;
      self.forward_read(&leader, action).await?
    };
    match result {
      ReadResult::ListResult(pairs) => Ok(
        pairs
          .into_iter()
          .map(|(k, v)| (k[self.prefix.len()..].to_string(), v))
          .collect(),
      ),
      ReadResult::Error(err) => Err(err.into()),
      _ => unreachable!("List always returns ListResult or Error"),
    }
  }
}

impl Barn {
  /// Proposes `action` to the raft log — locally if this node is the
  /// leader, or by forwarding to the resolved leader otherwise. Shared by
  /// `ItemStore::set`/`delete`.
  async fn write(&self, action: Action) -> Result<(), Box<dyn Error>> {
    let result = if self.is_leader().await {
      self.local_write(action).await
    } else {
      let leader = self
        .current_leader_node()
        .await
        .ok_or("no known raft leader")?;
      self.forward_write(&leader, action).await?
    };
    match result {
      ActionResult::Success => Ok(()),
      ActionResult::Error(err) => Err(err.into()),
    }
  }
}

#[async_trait::async_trait]
impl SpreadStore for Barn {
  type Node = raft::Node;

  async fn node(&self) -> Result<Option<raft::Node>, Box<dyn Error>> {
    Ok(
      self
        .node_cache
        .read()
        .await
        .iter()
        .find(|node| node.id == self.self_node.id)
        .cloned(),
    )
  }

  async fn node_list(&self) -> Result<Vec<raft::Node>, Box<dyn Error>> {
    Ok(self.node_cache.read().await.clone())
  }

  /// The first ever call (empty membership) bootstraps a brand-new
  /// single-node cluster via `raft.initialize`. Every later call adds
  /// `node` as a learner (so it starts replicating) and then promotes it
  /// to voter. Bootstrapping a fresh cluster vs. adding to an existing one
  /// is decided here, from raft's own membership state — not from any
  /// config the caller passes in.
  async fn node_add(&self, node: raft::Node) -> Result<(), Box<dyn Error>> {
    let metrics = self.raft.metrics().borrow().clone();
    if metrics.membership_config.nodes().next().is_none() {
      let mut members = std::collections::BTreeMap::new();
      members.insert(node.id, node);
      self.raft.initialize(members).await?;
    } else {
      self.raft.add_learner(node.id, node.clone(), true).await?;
      self
        .raft
        .change_membership(
          openraft::ChangeMembers::AddVoterIds([node.id].into()),
          true,
        )
        .await?;
    }
    Ok(())
  }

  async fn node_remove(&self, id: u64) -> Result<(), Box<dyn Error>> {
    // Demote first (no-op / ignorable error if it was only ever a learner),
    // then remove the node entry entirely.
    let _ = self
      .raft
      .change_membership(
        openraft::ChangeMembers::RemoveVoters([id].into()),
        true,
      )
      .await;
    self
      .raft
      .change_membership(
        openraft::ChangeMembers::RemoveNodes([id].into()),
        true,
      )
      .await?;
    Ok(())
  }

  /// Overwrites `node_cache` wholesale — the same cache `get`/`set`/
  /// `stale_get`/`node`/`node_list` already read from. For this
  /// raft-participating `Barn`, `event_forwarder_start` keeps the cache
  /// current on its own and nothing calls this; it exists so a future
  /// worker-mode `Barn` (no local raft participation) can be kept current
  /// by whoever pushes it cluster membership instead.
  async fn node_cache_set(
    &self,
    nodes: Vec<raft::Node>,
  ) -> Result<(), Box<dyn Error>> {
    *self.node_cache.write().await = nodes;
    Ok(())
  }
}

impl ClusterStore for Barn {
  type ClusterEvent = Event;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::croft::Gate;
  use std::collections::BTreeMap;
  use std::net::SocketAddr;
  use std::time::Duration;
  use tokio::time::{sleep, timeout};

  fn test_raft_config() -> (u64, u64, u64) {
    (50, 150, 300)
  }

  async fn spawn_test_barn(id: u64) -> Barn {
    let dir = tempfile::tempdir().expect("create temp dir");
    let (heartbeat, election_min, election_max) = test_raft_config();
    let config = Config {
      id,
      addr: format!("127.0.0.1:{id}"),
      data_dir: dir.path().to_path_buf(),
      heartbeat_interval: heartbeat,
      election_timeout_min: election_min,
      election_timeout_max: election_max,
    };
    // Leak the tempdir so its files outlive the test's `Storage`/raft
    // handles instead of being deleted while still open.
    std::mem::forget(dir);
    let gate = Gate::new();
    Barn::spawn(&gate, config).await.expect("spawn barn")
  }

  /// Reserves a free local port by binding to it and immediately dropping
  /// the listener.
  fn reserve_local_addr() -> SocketAddr {
    let listener =
      std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    listener.local_addr().expect("local addr")
  }

  /// Spawns a real `Barn` backed by a real `Gate::listen` on an actual
  /// local TCP port, so other nodes in the same test can reach it over
  /// real gRPC.
  async fn spawn_networked_test_barn(id: u64) -> (Arc<Barn>, SocketAddr) {
    let dir = tempfile::tempdir().expect("create temp dir");
    let addr = reserve_local_addr();
    let (heartbeat, election_min, election_max) = test_raft_config();
    let config = Config {
      id,
      addr: addr.to_string(),
      data_dir: dir.path().to_path_buf(),
      heartbeat_interval: heartbeat,
      election_timeout_min: election_min,
      election_timeout_max: election_max,
    };
    std::mem::forget(dir);

    let gate = Gate::new();
    let barn = Arc::new(Barn::spawn(&gate, config).await.expect("spawn barn"));
    tokio::spawn(async move {
      let _ = gate.listen(addr).await;
    });

    (barn, addr)
  }

  /// Polls `cond` until it returns `true`, failing the test instead of
  /// hanging forever if it never does. Needed because `node_cache` is
  /// refreshed by `event_forwarder_start`'s background task, not
  /// synchronously by whatever triggered the raft metrics change.
  async fn wait_until(mut cond: impl FnMut() -> bool) {
    timeout(Duration::from_secs(10), async {
      while !cond() {
        sleep(Duration::from_millis(20)).await;
      }
    })
    .await
    .expect("condition met within timeout");
  }

  #[test]
  fn nodes_from_metrics_marks_voters_and_the_current_leader() {
    let mut membership = BTreeMap::new();
    membership.insert(
      1,
      raft::Node {
        id: 1,
        api_addr: "a".to_string(),
        role: raft::NodeRole::Learner,
        is_leader: false,
      },
    );
    membership.insert(
      2,
      raft::Node {
        id: 2,
        api_addr: "b".to_string(),
        role: raft::NodeRole::Learner,
        is_leader: false,
      },
    );
    let stored = openraft::StoredMembership::new(
      None,
      openraft::Membership::new(
        vec![std::collections::BTreeSet::from([1])],
        membership,
      ),
    );
    let metrics = openraft::RaftMetrics::<raft::NodeId, raft::Node> {
      running_state: Ok(()),
      id: 1,
      current_term: 1,
      vote: openraft::Vote::new(1, 1u64),
      last_log_index: None,
      last_applied: None,
      snapshot: None,
      purged: None,
      state: openraft::ServerState::Leader,
      current_leader: Some(1),
      millis_since_quorum_ack: None,
      membership_config: Arc::new(stored),
      replication: None,
    };

    let mut nodes = Barn::nodes_from_metrics(&metrics);
    nodes.sort_by_key(|n| n.id);

    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].role, raft::NodeRole::Voter);
    assert!(nodes[0].is_leader);
    assert_eq!(nodes[1].role, raft::NodeRole::Learner);
    assert!(!nodes[1].is_leader);
  }

  #[tokio::test]
  async fn fresh_barn_has_no_leader() {
    let barn = spawn_test_barn(1).await;

    assert!(!barn.is_leader().await);
    assert!(barn.current_leader_node().await.is_none());
    assert!(barn.random_node().await.is_none());
  }

  #[tokio::test]
  async fn bootstrapped_single_node_cluster_is_its_own_leader() {
    let barn = spawn_test_barn(1).await;
    let mut members = BTreeMap::new();
    members.insert(barn.self_node.id, barn.self_node.clone());

    barn.raft().initialize(members).await.expect("initialize");
    wait_until(|| {
      barn
        .node_cache
        .try_read()
        .map(|nodes| !nodes.is_empty())
        .unwrap_or(false)
    })
    .await;

    assert!(barn.is_leader().await);
    assert_eq!(
      barn.current_leader_node().await.map(|n| n.id),
      Some(barn.self_node.id)
    );
    assert_eq!(
      barn.random_node().await.map(|n| n.id),
      Some(barn.self_node.id)
    );
  }

  /// Waits (with an overall timeout) for an event satisfying `pred` to
  /// arrive on `events`, discarding any that don't match — e.g. the
  /// occasional `NodeChanged` a heartbeat can produce alongside the
  /// `NodeAdded`/item event a test actually cares about.
  async fn recv_matching(
    events: &mut broadcast::Receiver<StoreEvent<Event>>,
    pred: impl Fn(&StoreEvent<Event>) -> bool,
  ) -> StoreEvent<Event> {
    timeout(Duration::from_secs(10), async {
      loop {
        let event = events.recv().await.expect("event channel open");
        if pred(&event) {
          return event;
        }
      }
    })
    .await
    .expect("matching event received within timeout")
  }

  #[tokio::test]
  async fn two_node_cluster_replicates_writes_and_emits_events_over_real_grpc()
  {
    let (node1, _addr1) = spawn_networked_test_barn(1).await;
    let (node2, _addr2) = spawn_networked_test_barn(2).await;

    // Bootstrap node1 as a single-node cluster, then wait for it to
    // become its own leader.
    node1
      .node_add(node1.self_node.clone())
      .await
      .expect("bootstrap node1");
    wait_until(|| {
      node1
        .node_cache
        .try_read()
        .map(|nodes| nodes.iter().any(|n| n.is_leader))
        .unwrap_or(false)
    })
    .await;

    let mut node1_events = node1.subscribe().await;

    // Add node2 to the (real, over-the-wire) cluster and wait for both
    // nodes' node_cache to agree on a 2-member membership — node2's own
    // cache only updates once it actually receives that membership
    // change from node1 over real gRPC.
    node1
      .node_add(node2.self_node.clone())
      .await
      .expect("add node2");
    wait_until(|| {
      node1
        .node_cache
        .try_read()
        .map(|nodes| nodes.len() == 2)
        .unwrap_or(false)
    })
    .await;
    wait_until(|| {
      node2
        .node_cache
        .try_read()
        .map(|nodes| nodes.len() == 2)
        .unwrap_or(false)
    })
    .await;

    let node_added = recv_matching(&mut node1_events, |event| {
      matches!(
        event,
        StoreEvent::Custom(Event::NodeAdded(node)) if node.id == 2
      )
    })
    .await;
    assert!(matches!(
      node_added,
      StoreEvent::Custom(Event::NodeAdded(_))
    ));

    // Write on node1 (the leader), read it back from node2 through the
    // default, always-consistent path — node2 isn't leader, so this
    // exercises `get`'s real gRPC forward-to-leader path.
    node1
      .set(&"foo".to_string(), b"bar".to_vec())
      .await
      .expect("set on node1");

    let node_created = recv_matching(&mut node1_events, |event| {
      matches!(event, StoreEvent::Custom(Event::ItemCreated(k, _)) if k == "foo")
    })
    .await;
    assert!(matches!(
      node_created,
      StoreEvent::Custom(Event::ItemCreated(_, _))
    ));

    let value = node2.get(&"foo".to_string()).await.expect("get from node2");
    assert_eq!(value, Some(b"bar".to_vec()));

    // stale_get from node2 may land on either node2's own (possibly
    // still-replicating) local copy or node1's, depending on the random
    // pick — retry until replication has caught up either way.
    wait_until(|| {
      node2
        .node_cache
        .try_read()
        .map(|nodes| !nodes.is_empty())
        .unwrap_or(false)
    })
    .await;
    let stale_value = timeout(Duration::from_secs(10), async {
      loop {
        if let Ok(Some(value)) = node2.stale_get("foo").await {
          return value;
        }
        sleep(Duration::from_millis(20)).await;
      }
    })
    .await
    .expect("stale_get eventually observes the write");
    assert_eq!(stale_value, b"bar".to_vec());
  }
}
