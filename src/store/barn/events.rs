//! Event definitions and conversions emitted by the Barn storage engine.
//!
//! Exposes [`Event`], uniting item lifecycle mutations and Raft membership
//! updates into a unified stream for subscribers.

use crate::store::{ItemStoreEvent, SpreadStoreEvent};

use super::raft;

/// The event type Barn emits. A flat enum rather than a wrapper around
/// `ItemStoreEvent`/`SpreadStoreEvent` — the `TryFrom` impls below
/// reconstruct the generic wrapped shape only where `RanchStore`'s
/// `TryInto` bounds require it.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
  ItemCreated(String, Vec<u8>),
  ItemPatched(String, Vec<u8>),
  ItemRemoved(String),
  NodeAdded(raft::Node),
  NodeRemoved(raft::Node),
  NodeChanged(raft::Node),
}

impl TryFrom<Event> for ItemStoreEvent<String, Vec<u8>> {
  type Error = ();

  fn try_from(event: Event) -> Result<Self, Self::Error> {
    match event {
      Event::ItemCreated(k, i) => Ok(ItemStoreEvent::ItemCreated(k, i)),
      Event::ItemPatched(k, i) => Ok(ItemStoreEvent::ItemPatched(k, i)),
      Event::ItemRemoved(k) => Ok(ItemStoreEvent::ItemRemoved(k)),
      _ => Err(()),
    }
  }
}

impl TryFrom<Event> for SpreadStoreEvent<raft::Node> {
  type Error = ();

  fn try_from(event: Event) -> Result<Self, Self::Error> {
    match event {
      Event::NodeAdded(n) => Ok(SpreadStoreEvent::NodeAdded(n)),
      Event::NodeRemoved(n) => Ok(SpreadStoreEvent::NodeRemoved(n)),
      Event::NodeChanged(n) => Ok(SpreadStoreEvent::NodeChanged(n)),
      _ => Err(()),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn node() -> raft::Node {
    raft::Node {
      id: 1,
      api_addr: "127.0.0.1:0".to_string(),
      role: raft::NodeRole::Voter,
      is_leader: false,
    }
  }

  #[test]
  fn item_events_convert_to_item_store_event_and_fail_as_spread_event() {
    let created = Event::ItemCreated("k".to_string(), b"v".to_vec());
    assert_eq!(
      ItemStoreEvent::try_from(created.clone()),
      Ok(ItemStoreEvent::ItemCreated("k".to_string(), b"v".to_vec()))
    );
    assert_eq!(SpreadStoreEvent::<raft::Node>::try_from(created), Err(()));

    let patched = Event::ItemPatched("k".to_string(), b"v2".to_vec());
    assert_eq!(
      ItemStoreEvent::try_from(patched),
      Ok(ItemStoreEvent::ItemPatched("k".to_string(), b"v2".to_vec()))
    );

    let removed = Event::ItemRemoved("k".to_string());
    assert_eq!(
      ItemStoreEvent::try_from(removed),
      Ok(ItemStoreEvent::ItemRemoved("k".to_string()))
    );
  }

  #[test]
  fn node_events_convert_to_spread_store_event_and_fail_as_item_event() {
    let added = Event::NodeAdded(node());
    assert_eq!(
      SpreadStoreEvent::try_from(added.clone()),
      Ok(SpreadStoreEvent::NodeAdded(node()))
    );
    assert_eq!(ItemStoreEvent::<String, Vec<u8>>::try_from(added), Err(()));

    let removed = Event::NodeRemoved(node());
    assert_eq!(
      SpreadStoreEvent::try_from(removed),
      Ok(SpreadStoreEvent::NodeRemoved(node()))
    );

    let changed = Event::NodeChanged(node());
    assert_eq!(
      SpreadStoreEvent::try_from(changed),
      Ok(SpreadStoreEvent::NodeChanged(node()))
    );
  }
}
