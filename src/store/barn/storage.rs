use std::io::Cursor;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::{Action, ActionResult, TypeConfig};
use openraft::{
  storage::{RaftStateMachine, Snapshot},
  EntryPayload, LogId, OptionalSend, RaftSnapshotBuilder, RaftTypeConfig,
  SnapshotMeta, StorageError, StorageIOError, StoredMembership,
};
use redb::{ReadableDatabase, ReadableTable, TableDefinition};

/// Shorthand for this state machine's node id and node types, so method
/// signatures below don't have to spell out `<TypeConfig as
/// RaftTypeConfig>::NodeId` (and `::Node`) every time.
type NodeId = <TypeConfig as RaftTypeConfig>::NodeId;
type Node = <TypeConfig as RaftTypeConfig>::Node;

/// State-machine metadata that isn't itself application data (currently
/// just the last-applied log id and membership), keyed by name.
const META_TABLE_DEF: TableDefinition<&str, &[u8]> =
  TableDefinition::new("meta");

/// Application data written by `Action::Set`/`Delete`/`Patch`, keyed by
/// the caller-supplied key.
const DATA_TABLE_DEF: TableDefinition<&str, &[u8]> =
  TableDefinition::new("data");

const LAST_APPLIED_LOG_ID_META_KEY: &str = "last_applied_log_id";
const LAST_APPLIED_MEMBERSHIP_META_KEY: &str = "last_applied_membership";

/// The metadata (as JSON) and raw bytes of the most recently built or
/// installed snapshot, so [`RaftStateMachine::get_current_snapshot`]
/// survives a restart instead of forgetting the snapshot ever existed.
const CURRENT_SNAPSHOT_META_META_KEY: &str = "current_snapshot_meta";
const CURRENT_SNAPSHOT_DATA_META_KEY: &str = "current_snapshot_data";

/// This struct is mean to hold the barn storage.
#[derive(Clone)]
pub struct Storage {
  db: Arc<redb::Database>,
}

impl Storage {
  /// Opens `db`, creating [`META_TABLE_DEF`] and [`DATA_TABLE_DEF`] if
  /// they don't already exist.
  pub(crate) fn new(
    db: Arc<redb::Database>,
  ) -> Result<Self, StorageError<NodeId>> {
    let write_txn = db
      .begin_write()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;
    {
      write_txn
        .open_table(META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      write_txn
        .open_table(DATA_TABLE_DEF)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;

    Ok(Self { db })
  }

  /// This function applies a single `Action` (`Set`/`Delete`/`Patch`) to
  /// durable application data, stored in [`DATA_TABLE_DEF`] as opaque bytes
  /// keyed by the caller-supplied key.
  ///
  /// Takes the same `write_txn` [`Self::apply`] is already using for the
  /// batch, rather than opening its own, so this entry's effect commits
  /// atomically with `last_applied_log_id` and every other entry in the
  /// batch — a crash can't apply some of a batch and not the rest.
  ///
  /// The three actions differ only in how they treat the key's current
  /// presence:
  /// - `Set` always writes the key, whether or not it already existed
  ///   (create-or-replace).
  /// - `Delete` always removes it, and is not an error if it was already absent
  ///   — deleting a key that's already gone reaches the same end state as
  ///   deleting one that was there.
  /// - `Patch` requires the key to already exist, and replaces its value;
  ///   patching a key that doesn't exist fails with `ActionResult::Error`
  ///   instead of creating it. Since `value` here is opaque bytes rather than a
  ///   structured diff, "must already exist" is the only sensible generic
  ///   reading of "patch" — there's no byte-level merge to perform. If a real
  ///   merge (e.g. JSON merge patch) is wanted later, this is the one branch
  ///   that needs to change.
  fn apply_action(
    &self,
    write_txn: &redb::WriteTransaction,
    action: Action,
  ) -> Result<ActionResult, StorageError<NodeId>> {
    let mut data_table = write_txn
      .open_table(DATA_TABLE_DEF)
      .map_err(|err| StorageIOError::write_state_machine(&err))?;

    match action {
      Action::Set { key, value } => {
        data_table
          .insert(key.as_str(), value.as_slice())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;

        Ok(ActionResult::Success)
      }
      Action::Delete { key } => {
        data_table
          .remove(key.as_str())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;

        Ok(ActionResult::Success)
      }
      Action::Patch { key, value } => {
        let exists = data_table
          .get(key.as_str())
          .map_err(|err| StorageIOError::write_state_machine(&err))?
          .is_some();

        if !exists {
          return Ok(ActionResult::Error(format!(
            "cannot patch key {key:?}: it does not exist"
          )));
        }

        data_table
          .insert(key.as_str(), value.as_slice())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;

        Ok(ActionResult::Success)
      }
    }
  }
}

/// See documentation in
/// https://docs.rs/openraft/0.9.25/openraft/storage/trait.RaftSnapshotBuilder.html
impl RaftSnapshotBuilder<TypeConfig> for Storage {
  /// Packages the current application data into a self-contained snapshot
  /// that can be sent wholesale to a lagging follower (instead of a long
  /// run of individual log entries) or used to compact the log.
  ///
  /// The watermark (`last_log_id`/`last_membership`) and the data dump
  /// are read from [`META_TABLE_DEF`] and [`DATA_TABLE_DEF`] inside a
  /// single redb read transaction, not two separate reads — redb read
  /// transactions are a consistent point-in-time view, so this guarantees
  /// the snapshot's stated `last_log_id` actually matches the data bytes
  /// even if [`RaftStateMachine::apply`] runs concurrently. Reading the
  /// watermark via [`Self::applied_state`] instead would reopen a second,
  /// possibly-later transaction and risk exactly that mismatch.
  ///
  /// The built snapshot is also saved as this node's "current" snapshot
  /// (what [`RaftStateMachine::get_current_snapshot`] returns), since a
  /// snapshot that was built but immediately forgotten wouldn't be of much
  /// use.
  async fn build_snapshot(
    &mut self,
  ) -> Result<
    Snapshot<TypeConfig>,
    StorageError<<TypeConfig as RaftTypeConfig>::NodeId>,
  > {
    let (last_log_id, last_membership, data) = {
      let read_txn = self
        .db
        .begin_read()
        .map_err(|err| StorageIOError::read_state_machine(&err))?;
      let meta_table = read_txn
        .open_table(META_TABLE_DEF)
        .map_err(|err| StorageIOError::read_state_machine(&err))?;
      let data_table = read_txn
        .open_table(DATA_TABLE_DEF)
        .map_err(|err| StorageIOError::read_state_machine(&err))?;

      let last_log_id = meta_table
        .get(LAST_APPLIED_LOG_ID_META_KEY)
        .map_err(|err| StorageIOError::read_state_machine(&err))?
        .map(|value| {
          serde_json::from_slice::<LogId<NodeId>>(value.value())
            .map_err(|err| StorageIOError::read_state_machine(&err))
        })
        .transpose()?;

      let last_membership = meta_table
        .get(LAST_APPLIED_MEMBERSHIP_META_KEY)
        .map_err(|err| StorageIOError::read_state_machine(&err))?
        .map(|value| {
          serde_json::from_slice::<StoredMembership<NodeId, Node>>(
            value.value(),
          )
          .map_err(|err| StorageIOError::read_state_machine(&err))
        })
        .transpose()?
        .unwrap_or_default();

      let mut pairs = Vec::new();
      for row in data_table
        .range::<&str>(..)
        .map_err(|err| StorageIOError::read_state_machine(&err))?
      {
        let (key, value) =
          row.map_err(|err| StorageIOError::read_state_machine(&err))?;
        pairs.push((key.value().to_string(), value.value().to_vec()));
      }
      let data = serde_json::to_vec(&pairs)
        .map_err(|err| StorageIOError::read_state_machine(&err))?;

      (last_log_id, last_membership, data)
    };

    // Only needs to be unique enough to tell two snapshots apart; it's an
    // opaque id used for transfer bookkeeping, not something callers parse.
    let snapshot_id = format!(
      "{}-{}",
      last_log_id
        .as_ref()
        .map(|log_id| log_id.to_string())
        .unwrap_or_else(|| "none".to_string()),
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos(),
    );

    let meta = SnapshotMeta {
      last_log_id,
      last_membership,
      snapshot_id,
    };

    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;
    {
      let mut meta_table = write_txn
        .open_table(META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      let meta_data = serde_json::to_vec(&meta)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      meta_table
        .insert(CURRENT_SNAPSHOT_META_META_KEY, meta_data.as_slice())
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      meta_table
        .insert(CURRENT_SNAPSHOT_DATA_META_KEY, data.as_slice())
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;

    Ok(Snapshot {
      meta,
      snapshot: Box::new(Cursor::new(data)),
    })
  }
}

/// See documentation in
/// https://docs.rs/openraft/0.9.25/openraft/storage/trait.RaftStateMachine.html
impl RaftStateMachine<TypeConfig> for Storage {
  type SnapshotBuilder = Self;

  /// This function tells Raft how far this state machine has already caught up,
  /// so it knows what's left to do after a restart.
  ///
  /// This is read at startup, before anything is applied, and it answers
  /// two separate questions at once:
  ///
  /// - `Option<LogId<NodeId>>`: the id of the last log entry this state machine
  ///   has applied. Openraft compares this against the log to figure out which
  ///   entries (if any) still need to be replayed through `apply()` — entries
  ///   at or below this id must NOT be applied again, since business-data
  ///   changes like `Action::Set`/`Delete`/`Patch` aren't safe to double-apply.
  /// - `StoredMembership<NodeId, Node>`: the cluster membership config that was
  ///   in effect as of that point. This is tracked separately from the applied
  ///   log id above (see [`LAST_APPLIED_MEMBERSHIP_META_KEY`]) because
  ///   membership only changes when a membership-config entry is applied, so it
  ///   can lag behind the most recently applied entry; `StoredMembership`
  ///   carries its own `log_id()` recording exactly which entry last set it.
  ///
  /// Both values are read from [`META_TABLE_DEF`]. Neither having
  /// been written yet (a brand-new node) is a normal, valid state: it
  /// means nothing has been applied, so the log id is `None` and the
  /// membership is the empty default.
  async fn applied_state(
    &mut self,
  ) -> Result<
    (Option<LogId<NodeId>>, StoredMembership<NodeId, Node>),
    StorageError<NodeId>,
  > {
    let read_txn = self
      .db
      .begin_read()
      .map_err(|err| StorageIOError::read_state_machine(&err))?;
    let meta_table = read_txn
      .open_table(META_TABLE_DEF)
      .map_err(|err| StorageIOError::read_state_machine(&err))?;

    let last_applied_log_id = meta_table
      .get(LAST_APPLIED_LOG_ID_META_KEY)
      .map_err(|err| StorageIOError::read_state_machine(&err))?
      .map(|value| {
        serde_json::from_slice::<LogId<NodeId>>(value.value())
          .map_err(|err| StorageIOError::read_state_machine(&err))
      })
      .transpose()?;

    let last_membership = meta_table
      .get(LAST_APPLIED_MEMBERSHIP_META_KEY)
      .map_err(|err| StorageIOError::read_state_machine(&err))?
      .map(|value| {
        serde_json::from_slice::<StoredMembership<NodeId, Node>>(value.value())
          .map_err(|err| StorageIOError::read_state_machine(&err))
      })
      .transpose()?
      .unwrap_or_default();

    Ok((last_applied_log_id, last_membership))
  }

  /// This function applies a batch of already-committed log entries to durable
  /// state, one at a time and in order.
  ///
  /// This is the only place business logic actually runs. Raft guarantees
  /// an entry has already been replicated to (and agreed on by) a quorum
  /// before it ever reaches `apply` — there's no more voting or rollback
  /// past this point, so whatever happens here is final.
  ///
  /// Each entry is one of three kinds:
  /// - `Blank`: a no-op a newly elected leader commits to mark the start of its
  ///   term. Nothing to do besides acknowledging it.
  /// - `Normal(Action)`: real application data (`Set`/`Delete`/`Patch`).
  ///   Delegated to [`Self::apply_action`].
  /// - `Membership`: a change to cluster membership, committed through the log
  ///   exactly like any other entry. Recording it here is *storing* the new
  ///   membership for recovery purposes — Raft itself already treats a
  ///   membership change as in effect as soon as it's committed, well before
  ///   it's applied here.
  ///
  /// `last_applied_log_id` and (if the batch contained one) the latest
  /// membership are written once, after the whole batch, rather than
  /// after each entry — only the final value matters for
  /// [`Self::applied_state`], and writing both in the same transaction as
  /// the batch's own effects means a crash can't leave them disagreeing
  /// with what was actually applied.
  ///
  /// Returns one response per input entry, in the same order: this is
  /// what gets handed back to whoever proposed each entry (e.g. the
  /// caller of `Raft::client_write`).
  async fn apply<I>(
    &mut self,
    entries: I,
  ) -> Result<Vec<ActionResult>, StorageError<NodeId>>
  where
    I:
      IntoIterator<Item = <TypeConfig as RaftTypeConfig>::Entry> + OptionalSend,
    I::IntoIter: OptionalSend,
  {
    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;

    let mut responses = Vec::new();
    let mut last_log_id = None;
    let mut last_membership = None;

    for entry in entries {
      let response = match entry.payload {
        EntryPayload::Blank => ActionResult::Success,
        EntryPayload::Normal(action) => {
          self.apply_action(&write_txn, action)?
        }
        EntryPayload::Membership(membership) => {
          last_membership =
            Some(StoredMembership::new(Some(entry.log_id), membership));
          ActionResult::Success
        }
      };

      responses.push(response);
      last_log_id = Some(entry.log_id);
    }

    {
      let mut meta_table = write_txn
        .open_table(META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;

      if let Some(log_id) = &last_log_id {
        let data = serde_json::to_vec(log_id)
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
        meta_table
          .insert(LAST_APPLIED_LOG_ID_META_KEY, data.as_slice())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
      }

      if let Some(membership) = &last_membership {
        let data = serde_json::to_vec(membership)
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
        meta_table
          .insert(LAST_APPLIED_MEMBERSHIP_META_KEY, data.as_slice())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
      }
    }

    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;

    Ok(responses)
  }

  async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
    self.clone()
  }

  /// Hands the caller a blank, writable buffer to stream an incoming
  /// snapshot's bytes into.
  ///
  /// This node might be a follower falling behind, about to receive a
  /// snapshot from the leader over the network in chunks; this just
  /// allocates the destination for those chunks. Nothing is persisted
  /// here — the buffer only becomes durable state once it's handed back
  /// to [`Self::install_snapshot`] once fully received.
  async fn begin_receiving_snapshot(
    &mut self,
  ) -> Result<
    Box<<TypeConfig as RaftTypeConfig>::SnapshotData>,
    StorageError<<TypeConfig as RaftTypeConfig>::NodeId>,
  > {
    Ok(Box::new(Cursor::new(Vec::new())))
  }

  /// Replaces this node's entire application state with the contents of
  /// a fully-received snapshot — either one streamed from the leader (a
  /// lagging follower catching up in one shot instead of replaying a long
  /// run of log entries) or one this node built itself in
  /// [`Self::build_snapshot`].
  ///
  /// Every key in [`DATA_TABLE_DEF`] is dropped and replaced by exactly
  /// what the snapshot contains — this is a wholesale swap, not a merge.
  /// The applied watermark and membership move to match `meta`, and the
  /// snapshot is saved as the new "current" one, all in the same
  /// transaction as the data swap so a crash mid-install can't leave the
  /// data, the watermark, and the recorded snapshot disagreeing with each
  /// other.
  async fn install_snapshot(
    &mut self,
    meta: &SnapshotMeta<
      <TypeConfig as RaftTypeConfig>::NodeId,
      <TypeConfig as RaftTypeConfig>::Node,
    >,
    snapshot: Box<<TypeConfig as RaftTypeConfig>::SnapshotData>,
  ) -> Result<(), StorageError<<TypeConfig as RaftTypeConfig>::NodeId>> {
    let data = snapshot.into_inner();
    let pairs: Vec<(String, Vec<u8>)> = serde_json::from_slice(&data)
      .map_err(|err| StorageIOError::read_state_machine(&err))?;

    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;
    {
      let mut data_table = write_txn
        .open_table(DATA_TABLE_DEF)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      data_table
        .retain(|_, _| false)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      for (key, value) in &pairs {
        data_table
          .insert(key.as_str(), value.as_slice())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
      }

      let mut meta_table = write_txn
        .open_table(META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;

      if let Some(log_id) = &meta.last_log_id {
        let log_id_data = serde_json::to_vec(log_id)
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
        meta_table
          .insert(LAST_APPLIED_LOG_ID_META_KEY, log_id_data.as_slice())
          .map_err(|err| StorageIOError::write_state_machine(&err))?;
      }

      let membership_data = serde_json::to_vec(&meta.last_membership)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      meta_table
        .insert(LAST_APPLIED_MEMBERSHIP_META_KEY, membership_data.as_slice())
        .map_err(|err| StorageIOError::write_state_machine(&err))?;

      let meta_data = serde_json::to_vec(meta)
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      meta_table
        .insert(CURRENT_SNAPSHOT_META_META_KEY, meta_data.as_slice())
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
      meta_table
        .insert(CURRENT_SNAPSHOT_DATA_META_KEY, data.as_slice())
        .map_err(|err| StorageIOError::write_state_machine(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_state_machine(&err))?;

    Ok(())
  }

  /// Returns this node's most recently built or installed snapshot, if
  /// it has one, so openraft can send it to a follower that needs to
  /// catch up.
  ///
  /// Unlike [`Self::applied_state`] (which is always current as of the
  /// last `apply`), this can be arbitrarily stale — nothing here rebuilds
  /// a snapshot on demand. It just reports whatever
  /// [`Self::build_snapshot`] or [`Self::install_snapshot`] last saved
  /// under [`CURRENT_SNAPSHOT_META_META_KEY`]/
  /// [`CURRENT_SNAPSHOT_DATA_META_KEY`], which is `None` for a brand-new node
  /// that has never built or received one.
  async fn get_current_snapshot(
    &mut self,
  ) -> Result<
    Option<Snapshot<TypeConfig>>,
    StorageError<<TypeConfig as RaftTypeConfig>::NodeId>,
  > {
    let read_txn = self
      .db
      .begin_read()
      .map_err(|err| StorageIOError::read_state_machine(&err))?;
    let meta_table = read_txn
      .open_table(META_TABLE_DEF)
      .map_err(|err| StorageIOError::read_state_machine(&err))?;

    let meta = meta_table
      .get(CURRENT_SNAPSHOT_META_META_KEY)
      .map_err(|err| StorageIOError::read_state_machine(&err))?
      .map(|value| {
        serde_json::from_slice::<SnapshotMeta<NodeId, Node>>(value.value())
          .map_err(|err| StorageIOError::read_state_machine(&err))
      })
      .transpose()?;

    let Some(meta) = meta else {
      return Ok(None);
    };

    let data = meta_table
      .get(CURRENT_SNAPSHOT_DATA_META_KEY)
      .map_err(|err| StorageIOError::read_state_machine(&err))?
      .map(|value| value.value().to_vec())
      .unwrap_or_default();

    Ok(Some(Snapshot {
      meta,
      snapshot: Box::new(Cursor::new(data)),
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use openraft::testing::{log_id, membership_ent};
  use redb::backends::InMemoryBackend;
  use redb::Builder;
  use std::collections::BTreeSet;

  fn new_store() -> Storage {
    let db = Builder::new()
      .create_with_backend(InMemoryBackend::new())
      .expect("create in-memory redb database");
    Storage::new(Arc::new(db)).expect("create storage")
  }

  fn entry(
    index: u64,
    payload: EntryPayload<TypeConfig>,
  ) -> openraft::Entry<TypeConfig> {
    openraft::Entry {
      log_id: log_id::<NodeId>(1, 1, index),
      payload,
    }
  }

  fn set_entry(
    index: u64,
    key: &str,
    value: &[u8],
  ) -> openraft::Entry<TypeConfig> {
    entry(
      index,
      EntryPayload::Normal(Action::Set {
        key: key.to_string(),
        value: value.to_vec(),
      }),
    )
  }

  fn delete_entry(index: u64, key: &str) -> openraft::Entry<TypeConfig> {
    entry(
      index,
      EntryPayload::Normal(Action::Delete {
        key: key.to_string(),
      }),
    )
  }

  fn patch_entry(
    index: u64,
    key: &str,
    value: &[u8],
  ) -> openraft::Entry<TypeConfig> {
    entry(
      index,
      EntryPayload::Normal(Action::Patch {
        key: key.to_string(),
        value: value.to_vec(),
      }),
    )
  }

  /// White-box peek at the data table, since there's no public read API
  /// for application data yet — only `apply` (write) and snapshotting.
  fn stored_pairs(store: &Storage) -> Vec<(String, Vec<u8>)> {
    let read_txn = store.db.begin_read().expect("begin read txn");
    let table = read_txn
      .open_table(DATA_TABLE_DEF)
      .expect("open data table");
    table
      .range::<&str>(..)
      .expect("range data table")
      .map(|row| {
        let (key, value) = row.expect("read row");
        (key.value().to_string(), value.value().to_vec())
      })
      .collect()
  }

  #[tokio::test]
  async fn applied_state_on_empty_store_returns_none_and_default() {
    let mut store = new_store();

    let (applied_log_id, membership) =
      store.applied_state().await.expect("applied state");

    assert_eq!(applied_log_id, None);
    assert_eq!(membership, StoredMembership::default());
  }

  #[tokio::test]
  async fn apply_blank_entry_updates_watermark_without_touching_data() {
    let mut store = new_store();

    let responses = store
      .apply(vec![entry(1, EntryPayload::Blank)])
      .await
      .expect("apply");

    assert_eq!(responses, vec![ActionResult::Success]);
    let (applied_log_id, _) =
      store.applied_state().await.expect("applied state");
    assert_eq!(applied_log_id, Some(log_id::<NodeId>(1, 1, 1)));
    assert!(stored_pairs(&store).is_empty());
  }

  #[tokio::test]
  async fn apply_set_writes_data_and_returns_success() {
    let mut store = new_store();

    let responses = store
      .apply(vec![set_entry(1, "foo", b"bar")])
      .await
      .expect("apply");

    assert_eq!(responses, vec![ActionResult::Success]);
    assert_eq!(
      stored_pairs(&store),
      vec![("foo".to_string(), b"bar".to_vec())]
    );
  }

  #[tokio::test]
  async fn apply_delete_removes_existing_key() {
    let mut store = new_store();
    store
      .apply(vec![set_entry(1, "foo", b"bar")])
      .await
      .expect("apply set");

    let responses = store
      .apply(vec![delete_entry(2, "foo")])
      .await
      .expect("apply delete");

    assert_eq!(responses, vec![ActionResult::Success]);
    assert!(stored_pairs(&store).is_empty());
  }

  #[tokio::test]
  async fn apply_delete_on_missing_key_still_succeeds() {
    let mut store = new_store();

    let responses = store
      .apply(vec![delete_entry(1, "missing")])
      .await
      .expect("apply delete");

    assert_eq!(responses, vec![ActionResult::Success]);
  }

  #[tokio::test]
  async fn apply_patch_updates_existing_key() {
    let mut store = new_store();
    store
      .apply(vec![set_entry(1, "foo", b"bar")])
      .await
      .expect("apply set");

    let responses = store
      .apply(vec![patch_entry(2, "foo", b"baz")])
      .await
      .expect("apply patch");

    assert_eq!(responses, vec![ActionResult::Success]);
    assert_eq!(
      stored_pairs(&store),
      vec![("foo".to_string(), b"baz".to_vec())]
    );
  }

  #[tokio::test]
  async fn apply_patch_on_missing_key_fails_without_creating_it() {
    let mut store = new_store();

    let responses = store
      .apply(vec![patch_entry(1, "missing", b"baz")])
      .await
      .expect("apply patch");

    assert!(matches!(
      &responses[..],
      [ActionResult::Error(msg)] if msg.contains("missing")
    ));
    assert!(stored_pairs(&store).is_empty());
  }

  #[tokio::test]
  async fn apply_membership_entry_updates_membership_and_watermark() {
    let mut store = new_store();
    let config: Vec<BTreeSet<NodeId>> = vec![BTreeSet::from([1])];

    let responses = store
      .apply(vec![membership_ent::<TypeConfig>(1, 1, 1, config)])
      .await
      .expect("apply membership");

    assert_eq!(responses, vec![ActionResult::Success]);
    let (applied_log_id, membership) =
      store.applied_state().await.expect("applied state");
    assert_eq!(applied_log_id, Some(log_id::<NodeId>(1, 1, 1)));
    assert_eq!(membership.log_id(), &Some(log_id::<NodeId>(1, 1, 1)));
    assert_eq!(membership.voter_ids().collect::<Vec<_>>(), vec![1]);
  }

  #[tokio::test]
  async fn apply_batch_returns_responses_in_order_and_final_watermark_only() {
    let mut store = new_store();
    let batch = vec![
      set_entry(1, "a", b"1"),
      set_entry(2, "b", b"2"),
      set_entry(3, "c", b"3"),
    ];

    let responses = store.apply(batch).await.expect("apply batch");

    assert_eq!(responses, vec![ActionResult::Success; 3]);
    let (applied_log_id, _) =
      store.applied_state().await.expect("applied state");
    assert_eq!(applied_log_id, Some(log_id::<NodeId>(1, 1, 3)));
    let mut pairs = stored_pairs(&store);
    pairs.sort();
    assert_eq!(
      pairs,
      vec![
        ("a".to_string(), b"1".to_vec()),
        ("b".to_string(), b"2".to_vec()),
        ("c".to_string(), b"3".to_vec()),
      ]
    );
  }

  #[tokio::test]
  async fn build_snapshot_on_empty_store_has_no_data_and_no_log_id() {
    let mut store = new_store();

    let snapshot = store.build_snapshot().await.expect("build snapshot");

    assert_eq!(snapshot.meta.last_log_id, None);
    assert_eq!(snapshot.meta.last_membership, StoredMembership::default());
    let pairs: Vec<(String, Vec<u8>)> =
      serde_json::from_slice(snapshot.snapshot.get_ref())
        .expect("decode snapshot data");
    assert!(pairs.is_empty());
  }

  #[tokio::test]
  async fn build_snapshot_captures_applied_data_and_becomes_current() {
    let mut store = new_store();
    store
      .apply(vec![set_entry(1, "foo", b"bar")])
      .await
      .expect("apply");

    let snapshot = store.build_snapshot().await.expect("build snapshot");

    assert_eq!(snapshot.meta.last_log_id, Some(log_id::<NodeId>(1, 1, 1)));
    let pairs: Vec<(String, Vec<u8>)> =
      serde_json::from_slice(snapshot.snapshot.get_ref())
        .expect("decode snapshot data");
    assert_eq!(pairs, vec![("foo".to_string(), b"bar".to_vec())]);

    let current = store
      .get_current_snapshot()
      .await
      .expect("get current snapshot")
      .expect("snapshot exists");
    assert_eq!(current.meta, snapshot.meta);
    assert_eq!(current.snapshot.get_ref(), snapshot.snapshot.get_ref());
  }

  #[tokio::test]
  async fn get_current_snapshot_on_fresh_store_is_none() {
    let mut store = new_store();

    let current = store
      .get_current_snapshot()
      .await
      .expect("get current snapshot");

    assert!(current.is_none());
  }

  #[tokio::test]
  async fn begin_receiving_snapshot_returns_empty_buffer() {
    let mut store = new_store();

    let buffer = store
      .begin_receiving_snapshot()
      .await
      .expect("begin receiving snapshot");

    assert!(buffer.get_ref().is_empty());
  }

  #[tokio::test]
  async fn install_snapshot_replaces_data_wholesale_and_updates_watermark() {
    let mut store = new_store();
    // Pre-existing, unrelated data that installing a snapshot must wipe.
    store
      .apply(vec![set_entry(1, "stale", b"old")])
      .await
      .expect("apply stale data");

    let pairs = vec![("fresh".to_string(), b"new".to_vec())];
    let data = serde_json::to_vec(&pairs).expect("encode snapshot data");
    let meta = SnapshotMeta {
      last_log_id: Some(log_id::<NodeId>(2, 1, 5)),
      last_membership: StoredMembership::default(),
      snapshot_id: "test-snapshot".to_string(),
    };

    store
      .install_snapshot(&meta, Box::new(Cursor::new(data.clone())))
      .await
      .expect("install snapshot");

    assert_eq!(
      stored_pairs(&store),
      vec![("fresh".to_string(), b"new".to_vec())]
    );
    let (applied_log_id, _) =
      store.applied_state().await.expect("applied state");
    assert_eq!(applied_log_id, Some(log_id::<NodeId>(2, 1, 5)));

    let current = store
      .get_current_snapshot()
      .await
      .expect("get current snapshot")
      .expect("snapshot exists");
    assert_eq!(current.meta, meta);
    assert_eq!(current.snapshot.get_ref(), &data);
  }

  #[tokio::test]
  async fn snapshot_transfers_state_between_two_stores() {
    let mut source = new_store();
    source
      .apply(vec![set_entry(1, "a", b"1"), set_entry(2, "b", b"2")])
      .await
      .expect("apply to source");

    let snapshot = source.build_snapshot().await.expect("build snapshot");
    let meta = snapshot.meta.clone();

    let mut target = new_store();
    target
      .install_snapshot(&meta, snapshot.snapshot)
      .await
      .expect("install into target");

    let mut target_pairs = stored_pairs(&target);
    target_pairs.sort();
    assert_eq!(
      target_pairs,
      vec![
        ("a".to_string(), b"1".to_vec()),
        ("b".to_string(), b"2".to_vec()),
      ]
    );
    let (applied_log_id, _) =
      target.applied_state().await.expect("applied state");
    assert_eq!(applied_log_id, Some(log_id::<NodeId>(1, 1, 2)));
  }
}
