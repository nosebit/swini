//! Generic item storage abstractions and event definitions.
//!
//! Exposes [`ItemStore`], the fundamental trait for storing, retrieving, and
//! deleting entity items within Swini's storage subsystem, along with
//! [`ItemStoreEvent`].

use super::core::Store;
use std::error::Error;

/// Custom store event emitted by item stores on item lifecycle mutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemStoreEvent<K, I> {
  /// Emitted when a new item is stored.
  ItemCreated(K, I),
  /// Emitted when an existing item's contents are updated or replaced.
  ItemPatched(K, I),
  /// Emitted when an item is removed from the store.
  ItemRemoved(K),
}

/// Generic key-value store holding entity items.
#[async_trait::async_trait]
pub trait ItemStore: Store
where
  Self::Event: TryInto<ItemStoreEvent<Self::Key, Self::Item>>,
{
  /// The generic type of an item key.
  type Key: Clone + Send + Sync + 'static;

  /// The generic type of an entity item value.
  type Item: Clone + Send + Sync + 'static;

  /// Creates a scoped slice of this ItemStore where all keys are transparently
  /// prefixed with the given prefix string.
  fn slice(&self, prefix: &str) -> Self
  where
    Self: Sized;

  /// Retrieves an item by its key from the store.
  ///
  /// # Errors
  /// Returns an error if the underlying storage engine fails to read the key.
  async fn get(
    &self,
    key: &Self::Key,
  ) -> Result<Option<Self::Item>, Box<dyn Error>>;

  /// Sets an item for the specified key, creating or replacing it.
  ///
  /// # Errors
  /// Returns an error if the storage engine fails to persist the write.
  async fn set(
    &self,
    key: &Self::Key,
    item: Self::Item,
  ) -> Result<(), Box<dyn Error>>;

  /// Deletes an item from the store by its key.
  ///
  /// # Errors
  /// Returns an error if the storage engine fails to delete the key.
  async fn delete(&self, key: &Self::Key) -> Result<(), Box<dyn Error>>;

  /// Lists all key-item pairs whose keys start with the specified prefix.
  ///
  /// # Errors
  /// Returns an error if scanning or reading from storage fails.
  async fn list(
    &self,
    prefix: Option<&Self::Key>,
  ) -> Result<Vec<(Self::Key, Self::Item)>, Box<dyn Error>>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn item_store_event_variants() {
    let created =
      ItemStoreEvent::ItemCreated("croft/1".to_string(), vec![1, 2, 3]);
    assert_eq!(
      created,
      ItemStoreEvent::ItemCreated("croft/1".to_string(), vec![1, 2, 3])
    );

    let patched =
      ItemStoreEvent::ItemPatched("croft/1".to_string(), vec![4, 5, 6]);
    assert_eq!(
      patched,
      ItemStoreEvent::ItemPatched("croft/1".to_string(), vec![4, 5, 6])
    );

    let removed: ItemStoreEvent<String, Vec<u8>> =
      ItemStoreEvent::ItemRemoved("croft/1".to_string());
    assert_eq!(removed, ItemStoreEvent::ItemRemoved("croft/1".to_string()));
  }
}
