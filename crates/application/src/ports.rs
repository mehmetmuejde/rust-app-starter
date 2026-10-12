//! Ports: the interfaces the application needs from the outside world. Adapters implement them.

use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use starter_domain::{Item, ItemId, Timestamp};

/// Failure inside a storage adapter.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StorageError {
    #[error("storage backend error: {0}")]
    Backend(String),
    #[error("stored data is corrupt: {0}")]
    Corrupt(String),
    #[error("an item with id {0} already exists")]
    Duplicate(String),
    #[error("item {0} does not exist")]
    Missing(String),
    #[error("the data was written by a newer version (schema {found}, this build supports {supported})")]
    NewerSchema { found: i64, supported: i64 },
}

/// Persistence of items.
///
/// Synchronous on purpose: adapters that are asynchronous underneath (IndexedDB) keep an in-memory
/// state and persist in the background. Every call is one unit of work; adapters wrap it in a
/// transaction where the backend supports it.
pub trait ItemRepository {
    /// Stores a new item. Fails with [`StorageError::Duplicate`] if the id exists.
    fn insert(&mut self, item: &Item) -> Result<(), StorageError>;
    /// Replaces an existing item. Fails with [`StorageError::Missing`] if the id does not exist.
    fn update(&mut self, item: &Item) -> Result<(), StorageError>;
    /// Removes an item. Returns `false` if it did not exist.
    fn delete(&mut self, id: ItemId) -> Result<bool, StorageError>;
    fn get(&self, id: ItemId) -> Result<Option<Item>, StorageError>;
    /// All items, oldest first (by `created_at`, then id).
    fn list(&self) -> Result<Vec<Item>, StorageError>;
    /// Human-readable location, e.g. `SQLite: /home/me/.local/share/rust-app/items.sqlite3`.
    fn describe(&self) -> String;
}

/// Source of the current time. Injected so tests are deterministic.
pub trait Clock {
    fn now(&self) -> Timestamp;
}

/// Wall clock. Uses `web-time`, so it works in the browser as well.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        let millis = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or(0);
        Timestamp::from_millis(millis)
    }
}

/// Clock under test control. Clones share the same time.
#[derive(Debug, Clone, Default)]
pub struct ManualClock(Arc<AtomicI64>);

impl ManualClock {
    pub fn new(start: Timestamp) -> Self {
        Self(Arc::new(AtomicI64::new(start.as_millis())))
    }

    pub fn set(&self, now: Timestamp) {
        self.0.store(now.as_millis(), Ordering::SeqCst);
    }

    pub fn advance_millis(&self, millis: i64) {
        self.0.fetch_add(millis, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        Timestamp::from_millis(self.0.load(Ordering::SeqCst))
    }
}
