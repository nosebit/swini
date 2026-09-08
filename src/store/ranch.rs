//! Ranch-level combined storage trait uniting item storage with consensus
//! replication.
//!
//! Exposes [`RanchStore`], representing the complete replicated storage engine
//! for entities across the Ranch.

use super::ItemStore;
use super::ItemStoreEvent;
use super::SpreadStore;
use super::SpreadStoreEvent;
use super::Store;

/// Trait defining a store that combines item storage and distributed spread
/// replication on the Ranch.
pub trait RanchStore:
  Store<Event = Self::RanchEvent>
  + ItemStore<Key = String, Item = Vec<u8>>
  + SpreadStore
{
  /// The composite event type emitted by this RanchStore.
  type RanchEvent: Clone
    + Send
    + Sync
    + 'static
    + TryInto<ItemStoreEvent<String, Vec<u8>>>
    + TryInto<SpreadStoreEvent<Self::Node>>;
}
