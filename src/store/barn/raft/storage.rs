use openraft::storage::{LogFlushed, RaftLogStorage};
use openraft::{
  LogId, LogState, OptionalSend, RaftLogId, RaftLogReader, RaftTypeConfig,
  StorageError, StorageIOError, Vote,
};
use redb::{ReadableDatabase, TableDefinition};
use std::marker::PhantomData;
use std::sync::Arc;

/// Log entries, keyed by their log index.
pub const RAFT_LOGS_TABLE_DEF: TableDefinition<u64, &[u8]> =
  TableDefinition::new(":raft-logs");

/// Storage metadata that isn't itself a log entry (e.g. the last purged log
/// id, and eventually the saved vote), keyed by name.
pub const RAFT_META_TABLE_DEF: TableDefinition<&str, &[u8]> =
  TableDefinition::new(":raft-meta");

const LAST_PURGED_LOG_ID_META_KEY: &str = "last_purged_log_id";
const VOTE_META_KEY: &str = "vote";

pub struct LogStorage<C: RaftTypeConfig> {
  db: Arc<redb::Database>,
  _data: PhantomData<C>,
}

impl<C: RaftTypeConfig> LogStorage<C> {
  /// Opens `db`, creating [`RAFT_LOGS_TABLE_DEF`] and [`RAFT_META_TABLE_DEF`]
  /// if they don't already exist. Table layout is `LogStorage`'s own
  /// implementation detail, so callers only need to hand over a database,
  /// not know what's inside it.
  pub(crate) fn new(
    db: Arc<redb::Database>,
  ) -> Result<Self, StorageError<C::NodeId>> {
    let write_txn = db
      .begin_write()
      .map_err(|err| StorageIOError::write_logs(&err))?;
    {
      write_txn
        .open_table(RAFT_LOGS_TABLE_DEF)
        .map_err(|err| StorageIOError::write_logs(&err))?;
      write_txn
        .open_table(RAFT_META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_logs(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_logs(&err))?;

    Ok(Self {
      db,
      _data: PhantomData,
    })
  }

  /// Serializes `entries` into the logs table, keyed by log index, in a
  /// single durable transaction. Split out from [`Self::append`] so that
  /// method can report both success and failure through the
  /// [`LogFlushed`] callback without duplicating the write logic per
  /// branch.
  fn write_entries(
    &self,
    entries: impl IntoIterator<Item = C::Entry>,
  ) -> Result<(), StorageError<C::NodeId>> {
    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_logs(&err))?;
    {
      let mut logs_table = write_txn
        .open_table(RAFT_LOGS_TABLE_DEF)
        .map_err(|err| StorageIOError::write_logs(&err))?;
      for entry in entries {
        let index = entry.get_log_id().index;
        let data = serde_json::to_vec(&entry)
          .map_err(|err| StorageIOError::write_logs(&err))?;
        logs_table
          .insert(index, data.as_slice())
          .map_err(|err| StorageIOError::write_logs(&err))?;
      }
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_logs(&err))?;

    Ok(())
  }
}

impl<C: RaftTypeConfig> Clone for LogStorage<C> {
  fn clone(&self) -> Self {
    Self {
      db: self.db.clone(),
      _data: PhantomData,
    }
  }
}

/// See doc about RaftLogStorage in
/// https://docs.rs/openraft/0.9.25/openraft/storage/trait.RaftLogStorage.html
impl<C: RaftTypeConfig> RaftLogStorage<C> for LogStorage<C> {
  /// The log reader handed out by [`Self::get_log_reader`]. `LogStorage`
  /// reads its own entries, so it is its own reader; the two roles are
  /// distinguished by trait (write path vs. read path), not by type.
  type LogReader = Self;

  /// Reports where this node's local log currently stands, so Raft can pick
  /// up where it left off after a restart.
  ///
  /// Openraft calls this during startup (before any log is read or
  /// appended) to learn two things from durable storage: `last_log_id`, the
  /// id of the newest entry actually stored, and `last_purged_log_id`, the
  /// id of the newest entry ever purged after being applied. Together they
  /// tell Raft the full range of log indexes it can rely on `[last_purged +
  /// 1, last_log]`, which drives decisions like what to replicate to
  /// followers and whether a peer needs a snapshot instead of log entries.
  /// When the log is empty, `last_log_id` falls back to
  /// `last_purged_log_id`, since everything below that point is known to
  /// have existed and been applied even though it's no longer stored.
  async fn get_log_state(
    &mut self,
  ) -> Result<LogState<C>, StorageError<C::NodeId>> {
    let read_txn = self
      .db
      .begin_read()
      .map_err(|err| StorageIOError::read_logs(&err))?;
    let meta_table = read_txn
      .open_table(RAFT_META_TABLE_DEF)
      .map_err(|err| StorageIOError::read_logs(&err))?;
    let logs_table = read_txn
      .open_table(RAFT_LOGS_TABLE_DEF)
      .map_err(|err| StorageIOError::read_logs(&err))?;

    let last_purged_log_id = meta_table
      .get(LAST_PURGED_LOG_ID_META_KEY)
      .map_err(|err| StorageIOError::read_logs(&err))?
      .map(|value| {
        serde_json::from_slice::<LogId<C::NodeId>>(value.value())
          .map_err(|err| StorageIOError::read_logs(&err))
      })
      .transpose()?;

    let last_log_id = logs_table
      .range::<u64>(..)
      .map_err(|err| StorageIOError::read_logs(&err))?
      .next_back()
      .transpose()
      .map_err(|err| StorageIOError::read_logs(&err))?
      .map(|(_, value)| {
        serde_json::from_slice::<C::Entry>(value.value())
          .map_err(|err| StorageIOError::read_logs(&err))
      })
      .transpose()?
      .map(|entry| entry.get_log_id().clone())
      .or_else(|| last_purged_log_id.clone());

    Ok(LogState {
      last_purged_log_id,
      last_log_id,
    })
  }

  /// Hands out a reader for the replication path, kept separate from the
  /// `&mut self` write path used by `append`/`truncate`/`purge`.
  ///
  /// Openraft runs one replication task per follower, and each task reads
  /// committed entries independently to stream them to its follower. Those
  /// reads must not be serialized behind (or block) log writes on the
  /// leader, so openraft asks for a distinct reader handle per task instead
  /// of sharing the single `&mut self` used for writing. Here that handle
  /// is just a clone of this struct: both sides wrap the same
  /// `Arc<redb::Database>`, so cloning is cheap and every reader still sees
  /// the latest committed data through redb's own MVCC snapshots.
  async fn get_log_reader(&mut self) -> Self::LogReader {
    self.clone()
  }

  /// Persists the vote this node last cast, so it survives a restart.
  ///
  /// The vote records who this node currently believes is leader (or is
  /// itself campaigning to be) and for which term. Raft's safety depends on
  /// never voting twice in the same election after a crash, so this write
  /// must be durable on disk before returning — unlike `append`, there is
  /// no flush callback to defer that guarantee to.
  async fn save_vote(
    &mut self,
    vote: &Vote<C::NodeId>,
  ) -> Result<(), StorageError<C::NodeId>> {
    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_vote(&err))?;
    {
      let mut meta_table = write_txn
        .open_table(RAFT_META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_vote(&err))?;
      let data = serde_json::to_vec(vote)
        .map_err(|err| StorageIOError::write_vote(&err))?;
      meta_table
        .insert(VOTE_META_KEY, data.as_slice())
        .map_err(|err| StorageIOError::write_vote(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_vote(&err))?;

    Ok(())
  }

  /// Returns the last vote persisted by [`Self::save_vote`], so Raft can
  /// resume knowing what it already promised before a restart.
  async fn read_vote(
    &mut self,
  ) -> Result<Option<Vote<C::NodeId>>, StorageError<C::NodeId>> {
    let read_txn = self
      .db
      .begin_read()
      .map_err(|err| StorageIOError::read_vote(&err))?;
    let meta_table = read_txn
      .open_table(RAFT_META_TABLE_DEF)
      .map_err(|err| StorageIOError::read_vote(&err))?;

    let vote = meta_table
      .get(VOTE_META_KEY)
      .map_err(|err| StorageIOError::read_vote(&err))?
      .map(|value| {
        serde_json::from_slice::<Vote<C::NodeId>>(value.value())
          .map_err(|err| StorageIOError::read_vote(&err))
      })
      .transpose()?;

    Ok(vote)
  }

  /// Persists new log entries and reports completion through `callback`
  /// rather than only through the return value.
  ///
  /// Raft replicates by streaming entries to followers as soon as they're
  /// durable, without waiting for them to be applied to the state machine,
  /// so `append` and `callback` exist to make that write observable to two
  /// different consumers on two different schedules: a `LogReader` must be
  /// able to see the entries as soon as this method returns, while the
  /// callback tells the engine once they're actually safe on disk (which
  /// here happens to be the same instant, since redb's `commit` is
  /// synchronous and durable — but the two-signal shape lets other storage
  /// engines flush asynchronously without changing the trait).
  async fn append<I>(
    &mut self,
    entries: I,
    callback: LogFlushed<C>,
  ) -> Result<(), StorageError<C::NodeId>>
  where
    I: IntoIterator<Item = C::Entry> + OptionalSend,
    I::IntoIter: OptionalSend,
  {
    let result = self.write_entries(entries);

    match &result {
      Ok(()) => callback.log_io_completed(Ok(())),
      Err(err) => {
        callback.log_io_completed(Err(std::io::Error::other(err.to_string())))
      }
    }

    result
  }

  /// Deletes `log_id` and everything after it, to resolve a conflict with
  /// the leader's log (the leader's entries at and after `log_id` will be
  /// re-appended in their place).
  async fn truncate(
    &mut self,
    log_id: LogId<C::NodeId>,
  ) -> Result<(), StorageError<C::NodeId>> {
    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_logs(&err))?;
    {
      let mut logs_table = write_txn
        .open_table(RAFT_LOGS_TABLE_DEF)
        .map_err(|err| StorageIOError::write_logs(&err))?;
      logs_table
        .retain_in(log_id.index.., |_, _| false)
        .map_err(|err| StorageIOError::write_logs(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_logs(&err))?;

    Ok(())
  }

  /// Discards entries up to and including `log_id`, once they're known to
  /// be applied and no longer needed for replication.
  ///
  /// Unlike `truncate`, this isn't correcting an error — it's routine
  /// space reclamation. `last_purged_log_id` in the meta table has to move
  /// with it, since [`Self::get_log_state`] falls back to that value as
  /// `last_log_id` once the log entries that would otherwise supply it are
  /// gone.
  async fn purge(
    &mut self,
    log_id: LogId<C::NodeId>,
  ) -> Result<(), StorageError<C::NodeId>> {
    let write_txn = self
      .db
      .begin_write()
      .map_err(|err| StorageIOError::write_logs(&err))?;
    {
      let mut logs_table = write_txn
        .open_table(RAFT_LOGS_TABLE_DEF)
        .map_err(|err| StorageIOError::write_logs(&err))?;
      logs_table
        .retain_in(..=log_id.index, |_, _| false)
        .map_err(|err| StorageIOError::write_logs(&err))?;

      let mut meta_table = write_txn
        .open_table(RAFT_META_TABLE_DEF)
        .map_err(|err| StorageIOError::write_logs(&err))?;
      let data = serde_json::to_vec(&log_id)
        .map_err(|err| StorageIOError::write_logs(&err))?;
      meta_table
        .insert(LAST_PURGED_LOG_ID_META_KEY, data.as_slice())
        .map_err(|err| StorageIOError::write_logs(&err))?;
    }
    write_txn
      .commit()
      .map_err(|err| StorageIOError::write_logs(&err))?;

    Ok(())
  }
}

/// See doc about RaftLogReader in
/// https://docs.rs/openraft/0.9.25/openraft/storage/trait.RaftLogReader.html
impl<C: RaftTypeConfig> RaftLogReader<C> for LogStorage<C> {
  /// This function returns the log entries whose index falls in `range`,
  /// deserialized and in ascending order — the read counterpart to what
  /// [`RaftLogStorage::append`] writes.
  ///
  /// This is what replication actually runs on: each follower has its own
  /// replication task on the leader, and that task calls this repeatedly
  /// (through the reader handle [`RaftLogStorage::get_log_reader`] hands
  /// out, not through the mutable write handle) to pull whatever entries
  /// that follower is missing and stream them over the network.
  ///
  /// `range`'s bounds match the trait's own convention: inclusive start,
  /// exclusive end. An index inside `range` that isn't in the table
  /// (already purged, for instance) is simply absent from the result
  /// rather than an error — the trait documents this as expected, since a
  /// caller with a stale view of the log has no way to know in advance
  /// which indexes still exist.
  async fn try_get_log_entries<
    RB: std::ops::RangeBounds<u64> + Clone + std::fmt::Debug + OptionalSend,
  >(
    &mut self,
    range: RB,
  ) -> Result<Vec<C::Entry>, StorageError<C::NodeId>> {
    let read_txn = self
      .db
      .begin_read()
      .map_err(|err| StorageIOError::read_logs(&err))?;
    let logs_table = read_txn
      .open_table(RAFT_LOGS_TABLE_DEF)
      .map_err(|err| StorageIOError::read_logs(&err))?;

    let mut entries = Vec::new();
    for row in logs_table
      .range(range)
      .map_err(|err| StorageIOError::read_logs(&err))?
    {
      let (_, value) = row.map_err(|err| StorageIOError::read_logs(&err))?;
      let entry = serde_json::from_slice::<C::Entry>(value.value())
        .map_err(|err| StorageIOError::read_logs(&err))?;
      entries.push(entry);
    }

    Ok(entries)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use openraft::storage::RaftLogStorageExt;
  use openraft::testing::{blank_ent, log_id};
  use redb::backends::InMemoryBackend;
  use redb::Builder;
  use std::io::Cursor;

  openraft::declare_raft_types!(
    pub TestTypeConfig:
      D = (),
      R = (),
  );

  fn new_store() -> LogStorage<TestTypeConfig> {
    let db = Builder::new()
      .create_with_backend(InMemoryBackend::new())
      .expect("create in-memory redb database");
    LogStorage::new(Arc::new(db)).expect("create log storage")
  }

  /// White-box peek at the logs table, bypassing `RaftLogReader` (which
  /// isn't implemented yet), to assert which indexes `truncate`/`purge`
  /// actually left behind.
  fn stored_indexes(store: &LogStorage<TestTypeConfig>) -> Vec<u64> {
    let read_txn = store.db.begin_read().expect("begin read txn");
    let table = read_txn
      .open_table(RAFT_LOGS_TABLE_DEF)
      .expect("open logs table");
    table
      .range::<u64>(..)
      .expect("range logs table")
      .map(|entry| entry.expect("read entry").0.value())
      .collect()
  }

  fn entries(
    indexes: impl IntoIterator<Item = u64>,
  ) -> Vec<openraft::Entry<TestTypeConfig>> {
    indexes
      .into_iter()
      .map(|index| blank_ent::<TestTypeConfig>(1, 1, index))
      .collect()
  }

  #[tokio::test]
  async fn get_log_state_on_empty_store_returns_none() {
    let mut store = new_store();

    let state = store.get_log_state().await.expect("get log state");

    assert_eq!(state.last_purged_log_id, None);
    assert_eq!(state.last_log_id, None);
  }

  #[tokio::test]
  async fn get_log_state_reflects_last_appended_entry() {
    let mut store = new_store();

    store
      .blocking_append(entries(1..=3))
      .await
      .expect("append entries");
    let state = store.get_log_state().await.expect("get log state");

    assert_eq!(state.last_purged_log_id, None);
    assert_eq!(state.last_log_id, Some(log_id::<u64>(1, 1, 3)));
  }

  #[tokio::test]
  async fn get_log_state_falls_back_to_last_purged_when_log_empty() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=2))
      .await
      .expect("append entries");

    store.purge(log_id::<u64>(1, 1, 2)).await.expect("purge");
    let state = store.get_log_state().await.expect("get log state");

    assert_eq!(state.last_purged_log_id, Some(log_id::<u64>(1, 1, 2)));
    assert_eq!(state.last_log_id, Some(log_id::<u64>(1, 1, 2)));
  }

  #[tokio::test]
  async fn read_vote_on_empty_store_returns_none() {
    let mut store = new_store();

    let vote = store.read_vote().await.expect("read vote");

    assert_eq!(vote, None);
  }

  #[tokio::test]
  async fn save_vote_then_read_vote_round_trips() {
    let mut store = new_store();
    let vote = Vote::new(1, 1u64);

    store.save_vote(&vote).await.expect("save vote");
    let read = store.read_vote().await.expect("read vote");

    assert_eq!(read, Some(vote));
  }

  #[tokio::test]
  async fn save_vote_overwrites_previous_vote() {
    let mut store = new_store();
    store
      .save_vote(&Vote::new(1, 1u64))
      .await
      .expect("save first vote");

    let second = Vote::new(2, 1u64);
    store.save_vote(&second).await.expect("save second vote");
    let read = store.read_vote().await.expect("read vote");

    assert_eq!(read, Some(second));
  }

  #[tokio::test]
  async fn append_persists_entries_contiguously() {
    let mut store = new_store();

    store
      .blocking_append(entries(1..=5))
      .await
      .expect("append entries");

    assert_eq!(stored_indexes(&store), vec![1, 2, 3, 4, 5]);
  }

  #[tokio::test]
  async fn truncate_removes_entries_at_and_after_log_id() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=5))
      .await
      .expect("append entries");

    store
      .truncate(log_id::<u64>(1, 1, 3))
      .await
      .expect("truncate");

    assert_eq!(stored_indexes(&store), vec![1, 2]);
  }

  #[tokio::test]
  async fn truncate_from_first_index_empties_the_log() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=3))
      .await
      .expect("append entries");

    store
      .truncate(log_id::<u64>(1, 1, 1))
      .await
      .expect("truncate");

    assert!(stored_indexes(&store).is_empty());
  }

  #[tokio::test]
  async fn truncate_does_not_touch_last_purged_log_id() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=5))
      .await
      .expect("append entries");
    store.purge(log_id::<u64>(1, 1, 2)).await.expect("purge");

    store
      .truncate(log_id::<u64>(1, 1, 4))
      .await
      .expect("truncate");
    let state = store.get_log_state().await.expect("get log state");

    assert_eq!(state.last_purged_log_id, Some(log_id::<u64>(1, 1, 2)));
  }

  #[tokio::test]
  async fn purge_removes_entries_up_to_and_including_log_id() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=5))
      .await
      .expect("append entries");

    store.purge(log_id::<u64>(1, 1, 3)).await.expect("purge");

    assert_eq!(stored_indexes(&store), vec![4, 5]);
  }

  #[tokio::test]
  async fn purge_updates_last_purged_log_id() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=5))
      .await
      .expect("append entries");

    store.purge(log_id::<u64>(1, 1, 3)).await.expect("purge");
    let state = store.get_log_state().await.expect("get log state");

    assert_eq!(state.last_purged_log_id, Some(log_id::<u64>(1, 1, 3)));
  }

  #[tokio::test]
  async fn get_log_reader_shares_state_with_original_store() {
    let mut store = new_store();
    store
      .blocking_append(entries(1..=1))
      .await
      .expect("append entries");

    let mut reader = store.get_log_reader().await;
    let state = reader.get_log_state().await.expect("get log state");

    assert_eq!(state.last_log_id, Some(log_id::<u64>(1, 1, 1)));
  }

  #[tokio::test]
  async fn full_lifecycle_scenario() {
    let mut store = new_store();

    store
      .save_vote(&Vote::new(1, 1u64))
      .await
      .expect("save vote");
    store
      .blocking_append(entries(1..=5))
      .await
      .expect("append entries");

    store.purge(log_id::<u64>(1, 1, 2)).await.expect("purge");
    store
      .truncate(log_id::<u64>(1, 1, 4))
      .await
      .expect("truncate");

    assert_eq!(stored_indexes(&store), vec![3]);

    let state = store.get_log_state().await.expect("get log state");
    assert_eq!(state.last_purged_log_id, Some(log_id::<u64>(1, 1, 2)));
    assert_eq!(state.last_log_id, Some(log_id::<u64>(1, 1, 3)));

    let vote = store.read_vote().await.expect("read vote");
    assert_eq!(vote, Some(Vote::new(1, 1u64)));
  }
}
