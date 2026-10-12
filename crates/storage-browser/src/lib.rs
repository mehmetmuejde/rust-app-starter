//! [`ItemRepository`] for the browser.
//!
//! Browser storage APIs (IndexedDB, OPFS) are asynchronous, while the application core is
//! synchronous (see `docs/adr/0001-*`). This adapter therefore keeps the working state in memory
//! ([`InMemoryItemRepository`]) and, after every successful change, hands a JSON snapshot of all items
//! to a [`SnapshotStore`]. On wasm the store is IndexedDB: the write is issued immediately and
//! completes asynchronously in submission order.
//!
//! Trade-offs (documented in `docs/development/database.md`):
//! - A write is durable once the IndexedDB transaction completes (milliseconds later), not when
//!   `insert` returns. Closing the tab in that window can lose the last change.
//! - Every change rewrites the whole snapshot. Fine for thousands of records; switch to one IndexedDB
//!   record per item when data grows beyond that.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

#[cfg(target_arch = "wasm32")]
mod indexed_db;

#[cfg(target_arch = "wasm32")]
pub use indexed_db::IndexedDbSnapshotStore;

use serde::{Deserialize, Serialize};
use starter_application::{InMemoryItemRepository, ItemRepository, StorageError};
use starter_domain::{Item, ItemId};

/// Version of the snapshot format written by this build.
pub const SNAPSHOT_VERSION: u32 = 1;

/// Destination of snapshots. Implementations must apply saves in call order.
pub trait SnapshotStore {
    /// Persists the snapshot. May complete asynchronously; synchronous failures are returned.
    fn save(&mut self, snapshot_json: String) -> Result<(), StorageError>;
    /// Human-readable location for `describe()`.
    fn describe(&self) -> String;
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    items: Vec<Item>,
}

/// In-memory repository mirrored to a [`SnapshotStore`].
pub struct BrowserItemRepository {
    inner: InMemoryItemRepository,
    store: Box<dyn SnapshotStore>,
}

impl BrowserItemRepository {
    /// Restores the repository from a previously saved snapshot (`None` = first start).
    ///
    /// A snapshot from a newer build or with invalid content is refused instead of being overwritten.
    pub fn from_snapshot(snapshot_json: Option<&str>, store: Box<dyn SnapshotStore>) -> Result<Self, StorageError> {
        let inner = match snapshot_json {
            None => InMemoryItemRepository::new(),
            Some(json) => {
                let snapshot: Snapshot =
                    serde_json::from_str(json).map_err(|e| StorageError::Corrupt(format!("snapshot: {e}")))?;
                if snapshot.version > SNAPSHOT_VERSION {
                    return Err(StorageError::NewerSchema {
                        found: i64::from(snapshot.version),
                        supported: i64::from(SNAPSHOT_VERSION),
                    });
                }
                InMemoryItemRepository::from_items(snapshot.items)
            }
        };
        Ok(Self { inner, store })
    }

    /// Opens the IndexedDB database `db_name` and loads the last snapshot.
    #[cfg(target_arch = "wasm32")]
    pub async fn open_indexed_db(db_name: &str) -> Result<Self, StorageError> {
        let store = IndexedDbSnapshotStore::open(db_name).await?;
        let snapshot = store.load().await?;
        Self::from_snapshot(snapshot.as_deref(), Box::new(store))
    }

    /// Applies `change` and persists the result; on a synchronous save failure the change is undone.
    fn mutate<T>(
        &mut self,
        change: impl FnOnce(&mut InMemoryItemRepository) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        let before = self.inner.clone();
        let value = change(&mut self.inner)?;
        let snapshot = Snapshot {
            version: SNAPSHOT_VERSION,
            items: self.inner.list()?,
        };
        let json =
            serde_json::to_string(&snapshot).map_err(|e| StorageError::Backend(format!("encode snapshot: {e}")))?;
        if let Err(err) = self.store.save(json) {
            self.inner = before;
            return Err(err);
        }
        Ok(value)
    }
}

impl ItemRepository for BrowserItemRepository {
    fn insert(&mut self, item: &Item) -> Result<(), StorageError> {
        self.mutate(|repo| repo.insert(item))
    }

    fn update(&mut self, item: &Item) -> Result<(), StorageError> {
        self.mutate(|repo| repo.update(item))
    }

    fn delete(&mut self, id: ItemId) -> Result<bool, StorageError> {
        self.mutate(|repo| repo.delete(id))
    }

    fn get(&self, id: ItemId) -> Result<Option<Item>, StorageError> {
        self.inner.get(id)
    }

    fn list(&self) -> Result<Vec<Item>, StorageError> {
        self.inner.list()
    }

    fn describe(&self) -> String {
        self.store.describe()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use starter_application::contract;

    use super::*;

    /// Test store: keeps the last snapshot, optionally fails.
    #[derive(Clone, Default)]
    struct MemoryStore {
        last: Rc<RefCell<Option<String>>>,
        fail: Rc<RefCell<bool>>,
    }

    impl SnapshotStore for MemoryStore {
        fn save(&mut self, snapshot_json: String) -> Result<(), StorageError> {
            if *self.fail.borrow() {
                return Err(StorageError::Backend("quota exceeded".into()));
            }
            *self.last.borrow_mut() = Some(snapshot_json);
            Ok(())
        }

        fn describe(&self) -> String {
            "memory".into()
        }
    }

    #[test]
    fn satisfies_repository_contract() {
        contract::run_all(&mut || {
            Box::new(BrowserItemRepository::from_snapshot(None, Box::new(MemoryStore::default())).unwrap())
        });
    }

    #[test]
    fn snapshot_roundtrip_restores_items() {
        let store = MemoryStore::default();
        let mut repo = BrowserItemRepository::from_snapshot(None, Box::new(store.clone())).unwrap();
        let item = contract::item("kept", 7);
        repo.insert(&item).unwrap();
        let saved = store.last.borrow().clone().unwrap();
        let restored = BrowserItemRepository::from_snapshot(Some(&saved), Box::new(MemoryStore::default())).unwrap();
        assert_eq!(restored.get(item.id).unwrap(), Some(item));
    }

    #[test]
    fn failed_save_rolls_back() {
        let store = MemoryStore::default();
        let mut repo = BrowserItemRepository::from_snapshot(None, Box::new(store.clone())).unwrap();
        *store.fail.borrow_mut() = true;
        assert!(repo.insert(&contract::item("lost", 1)).is_err());
        assert!(repo.list().unwrap().is_empty());
    }

    #[test]
    fn newer_snapshot_is_refused() {
        let json = r#"{"version":99,"items":[]}"#;
        let err = BrowserItemRepository::from_snapshot(Some(json), Box::new(MemoryStore::default()))
            .err()
            .unwrap();
        assert!(matches!(err, StorageError::NewerSchema { found: 99, .. }));
    }

    #[test]
    fn garbage_snapshot_is_corrupt() {
        let err = BrowserItemRepository::from_snapshot(Some("{"), Box::new(MemoryStore::default()))
            .err()
            .unwrap();
        assert!(matches!(err, StorageError::Corrupt(_)));
    }
}
