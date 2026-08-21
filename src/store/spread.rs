use super::core::Store;

/// This is a custom store event that spread stores can emit.
#[derive(Debug, Clone, PartialEq)]
pub enum SpreadStoreEvent<N> {
  NodeAdded(N),
  NodeRemoved(N),
  NodeChanged(N),
}

/// This trait defines a single node of the spread store where
/// the underlying storage is replicated for high availability
pub trait SpreadNode: Clone + Send + Sync + 'static {
  fn new(id: u64, name: String, api_addr: String) -> Self;
  fn id(&self) -> u64;
  fn name(&self) -> String;
  fn api_addr(&self) -> String;
}

/// This trait defines a store which is replicated (spread) accross different
/// members.
#[async_trait::async_trait]
pub trait SpreadStore: Store
where
  Self::Event: TryInto<SpreadStoreEvent<Self::Node>>,
{
  type Node: SpreadNode;

  /// This function returns the current (local) node of the store.
  async fn node(
    &self,
  ) -> Result<Option<Self::Node>, Box<dyn std::error::Error>>;

  /// This function returns the list of all nodes holding copies of this
  /// store.
  async fn node_list(
    &self,
  ) -> Result<Vec<Self::Node>, Box<dyn std::error::Error>>;

  /// This function adds a new node to the spread store, making it eligible
  /// for receiving a copy of the store.
  async fn node_add(
    &self,
    node: Self::Node,
  ) -> Result<(), Box<dyn std::error::Error>>;

  /// This function removes a node from the spread.
  async fn node_remove(
    &self,
    id: u64,
  ) -> Result<(), Box<dyn std::error::Error>>;

  /// This functions updates the internal (cache) list of nodes.
  /// This is useful when the actual list of nodes is determined via some
  /// discovery mechanism. This is specially used in Raft based stores where
  /// a worker node not participating in the store distributing still needs to
  /// know the list of nodes to proxy requests to them (to the leader or to
  /// followers when stale data is allowed).
  async fn node_cache_set(
    &self,
    nodes: Vec<Self::Node>,
  ) -> Result<(), Box<dyn std::error::Error>>;
}
