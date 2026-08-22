# Store Module

This module defines and implements different types of stores used throughout the
swini cli.

We start defining the generic `Store` trait in `./core.rs` as an observable
struct capable of emitting `StoreEvent`. Every `Store` must emit at least the
`Initialized` event.

We define the `ItemStore` and `ItemStoreEvent` in `./item` to represent a
generic key-value store.

We define the `SpreadStore` and `SpreadStoreEvent` in `./spread` to represent a
store which is spread across multiple nodes.

We define the `ClusterStore` and `ClusterStoreEvent` in `./cluster` to represent
a store which is both an `ItemStore` and a `SpreadStore` at the same time.

This module also defines the following concrete implementations of stores:

- `barn`: This is a ClusterStore meant to be the main store used to hold the
  cluster state. It is a raft-based distributed key-value store.
