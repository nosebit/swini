/// This represents an event that any store can emit. By default any store must
/// emit at least the Initialized event but they can define custom events to be
/// emitted as well.
#[derive(Debug, Clone, PartialEq)]
pub enum StoreEvent<E> {
  /// All stores need to emit this event when its initialization is complete.
  Initialized,

  /// Stores can define custom events to be emitted as well.
  Custom(E),
}

/// This represents a store which simply expose a subscription mecanism.
#[async_trait::async_trait]
pub trait Store: Send + Sync + 'static {
  type Event: Clone + Send + Sync + 'static;

  async fn subscribe(
    &self,
  ) -> tokio::sync::broadcast::Receiver<StoreEvent<Self::Event>>;
}
