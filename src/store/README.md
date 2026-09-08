# Store Module

This module defines and implements different types of stores used throughout the
swini cli.

We start defining the generic `Store` trait in `./core.rs` as an observable
struct capable of emitting `StoreEvent`. Every `Store` must emit at least the
`Initialized` event.

We define the `ItemStore` and `ItemStoreEvent` in `./item.rs` to represent a
generic key-value store holding entity items.

We define the `SpreadStore` and `SpreadStoreEvent` in `./spread.rs` to represent
a store which is spread across multiple nodes.

We define the `RanchStore` in `./ranch.rs` to represent a store which is both an
`ItemStore` and a `SpreadStore` at the same time.

This module also defines the following concrete implementations of stores:

- `barn`: This is a RanchStore meant to be the main store used to hold the
  living entity records and cluster state. It is a raft-based distributed
  key-value store.
