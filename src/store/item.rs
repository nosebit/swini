use super::core::Store;
use std::error::Error;

/// This is a custom store event that item stores can emit.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemStoreEvent<K, V> {
  ItemCreated(K, V),
  ItemPatched(K, V),
  ItemRemoved(K),
}

/// This is a generic key-value store.
#[async_trait::async_trait]
pub trait ItemStore: Store
where
  Self::Event: TryInto<ItemStoreEvent<Self::Key, Self::Value>>,
{
  /// The generic type of an item key.
  type Key: Clone + Send + Sync + 'static;

  /// The generic type of an item value.
  type Value: Clone + Send + Sync + 'static;

  /// This function creates a slice of this ItemStore which basically
  /// creates a scope where all keys will be prefixed with the given prefix.
  fn slice(&self, prefix: &str) -> Self
  where
    Self: Sized;

  /// This function retrieves an item value given an item key.
  async fn get(
    &self,
    key: &Self::Key,
  ) -> Result<Option<Self::Value>, Box<dyn Error>>;

  /// This function sets an item value for a given item key.
  async fn set(
    &self,
    key: &Self::Key,
    value: Self::Value,
  ) -> Result<(), Box<dyn Error>>;

  // This function deletes an item from the store by its key.
  async fn delete(&self, key: &Self::Key) -> Result<(), Box<dyn Error>>;

  // This function lists all items whose keys are prefixed with the given
  // prefix.
  async fn list(
    &self,
    prefix: Option<&Self::Key>,
  ) -> Result<Vec<(Self::Key, Self::Value)>, Box<dyn Error>>;
}
