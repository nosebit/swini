use super::ItemStore;
use super::ItemStoreEvent;
use super::SpreadStore;
use super::SpreadStoreEvent;
use super::Store;

/// This trait simply defines a store which is an item store and
/// a spread store at the same time.
pub trait ClusterStore:
  Store<Event = Self::ClusterEvent>
  + ItemStore<Key = String, Value = Vec<u8>>
  + SpreadStore
{
  type ClusterEvent: Clone
    + Send
    + Sync
    + 'static
    + TryInto<ItemStoreEvent<String, Vec<u8>>>
    + TryInto<SpreadStoreEvent<Self::Node>>;
}
