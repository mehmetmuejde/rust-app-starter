//! [`ItemRepository`] on SQLite.
//!
//! - One database file per user (the host chooses the path), WAL journal mode.
//! - Schema migrations are plain SQL files embedded at compile time and tracked with
//!   `PRAGMA user_version`; a database written by a newer build is refused, never modified.
//! - Every repository call is its own transaction (SQLite's autocommit for single statements).
//!
//! Native only: the browser build uses `starter-storage-browser`.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};
use starter_application::{ItemRepository, StorageError};
use starter_domain::{Item, ItemId, ItemName, Timestamp};

/// Migrations in order. Index + 1 is the schema version they produce. Never edit a released file;
/// add a new one.
const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_items.sql")];

/// SQLite-backed item repository.
pub struct SqliteItemRepository {
    conn: Connection,
    location: String,
}

impl SqliteItemRepository {
    /// Opens (or creates) the database file and migrates it to the current schema.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| backend(&format!("create {}", parent.display()), e))?;
        }
        let conn = Connection::open(path).map_err(|e| backend("open database", e))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| backend("enable WAL", e))?;
        Self::init(conn, format!("SQLite: {}", path.display()))
    }

    /// Opens a private in-memory database (tests, `--in-memory` runs).
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory().map_err(|e| backend("open in-memory database", e))?;
        Self::init(conn, "SQLite: in memory (not persisted)".to_owned())
    }

    /// Default file name inside a data directory.
    pub fn default_path(data_dir: &Path) -> PathBuf {
        data_dir.join("items.sqlite3")
    }

    /// Current schema version of this build.
    pub fn schema_version() -> i64 {
        i64::try_from(MIGRATIONS.len()).unwrap_or(i64::MAX)
    }

    fn init(mut conn: Connection, location: String) -> Result<Self, StorageError> {
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| backend("enable foreign keys", e))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| backend("set busy timeout", e))?;
        migrate(&mut conn)?;
        Ok(Self { conn, location })
    }
}

fn migrate(conn: &mut Connection) -> Result<(), StorageError> {
    let current: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| backend("read schema version", e))?;
    let supported = SqliteItemRepository::schema_version();
    if current > supported {
        return Err(StorageError::NewerSchema {
            found: current,
            supported,
        });
    }
    for (index, sql) in MIGRATIONS
        .iter()
        .enumerate()
        .skip(usize::try_from(current).unwrap_or(usize::MAX))
    {
        let version = i64::try_from(index + 1).unwrap_or(i64::MAX);
        let tx = conn.transaction().map_err(|e| backend("begin migration", e))?;
        tx.execute_batch(sql)
            .map_err(|e| backend(&format!("apply migration {version}"), e))?;
        tx.pragma_update(None, "user_version", version)
            .map_err(|e| backend("write schema version", e))?;
        tx.commit().map_err(|e| backend("commit migration", e))?;
        log::info!("database migrated to schema version {version}");
    }
    Ok(())
}

fn backend(context: &str, err: impl std::fmt::Display) -> StorageError {
    StorageError::Backend(format!("{context}: {err}"))
}

fn item_from_row(row: &Row<'_>) -> rusqlite::Result<Result<Item, StorageError>> {
    let id: String = row.get(0)?;
    let name: String = row.get(1)?;
    let created_at: i64 = row.get(2)?;
    let updated_at: i64 = row.get(3)?;
    Ok(decode_item(&id, &name, created_at, updated_at))
}

fn decode_item(id: &str, name: &str, created_at: i64, updated_at: i64) -> Result<Item, StorageError> {
    let id: ItemId = id
        .parse()
        .map_err(|_| StorageError::Corrupt(format!("invalid item id '{id}'")))?;
    let name = ItemName::parse(name).map_err(|e| StorageError::Corrupt(format!("item {id}: {e}")))?;
    Ok(Item {
        id,
        name,
        created_at: Timestamp::from_millis(created_at),
        updated_at: Timestamp::from_millis(updated_at),
    })
}

impl ItemRepository for SqliteItemRepository {
    fn insert(&mut self, item: &Item) -> Result<(), StorageError> {
        let result = self.conn.execute(
            "INSERT INTO items (id, name, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                item.id.to_string(),
                item.name.as_str(),
                item.created_at.as_millis(),
                item.updated_at.as_millis()
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
                Err(StorageError::Duplicate(item.id.to_string()))
            }
            Err(e) => Err(backend("insert item", e)),
        }
    }

    fn update(&mut self, item: &Item) -> Result<(), StorageError> {
        let changed = self
            .conn
            .execute(
                "UPDATE items SET name = ?2, created_at = ?3, updated_at = ?4 WHERE id = ?1",
                params![
                    item.id.to_string(),
                    item.name.as_str(),
                    item.created_at.as_millis(),
                    item.updated_at.as_millis()
                ],
            )
            .map_err(|e| backend("update item", e))?;
        if changed == 0 {
            return Err(StorageError::Missing(item.id.to_string()));
        }
        Ok(())
    }

    fn delete(&mut self, id: ItemId) -> Result<bool, StorageError> {
        let changed = self
            .conn
            .execute("DELETE FROM items WHERE id = ?1", params![id.to_string()])
            .map_err(|e| backend("delete item", e))?;
        Ok(changed > 0)
    }

    fn get(&self, id: ItemId) -> Result<Option<Item>, StorageError> {
        self.conn
            .query_row(
                "SELECT id, name, created_at, updated_at FROM items WHERE id = ?1",
                params![id.to_string()],
                item_from_row,
            )
            .optional()
            .map_err(|e| backend("read item", e))?
            .transpose()
    }

    fn list(&self) -> Result<Vec<Item>, StorageError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT id, name, created_at, updated_at FROM items ORDER BY created_at, id")
            .map_err(|e| backend("prepare list", e))?;
        let rows = stmt
            .query_map([], item_from_row)
            .map_err(|e| backend("list items", e))?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| backend("read row", e))??);
        }
        Ok(items)
    }

    fn describe(&self) -> String {
        self.location.clone()
    }
}

#[cfg(test)]
mod tests {
    use starter_application::contract;

    use super::*;

    #[test]
    fn satisfies_repository_contract() {
        contract::run_all(&mut || Box::new(SqliteItemRepository::open_in_memory().unwrap()));
    }

    #[test]
    fn data_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = SqliteItemRepository::default_path(dir.path());
        let item = contract::item("persisted", 42);
        SqliteItemRepository::open(&path).unwrap().insert(&item).unwrap();
        let reopened = SqliteItemRepository::open(&path).unwrap();
        assert_eq!(reopened.get(item.id).unwrap(), Some(item));
    }

    #[test]
    fn newer_schema_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let path = SqliteItemRepository::default_path(dir.path());
        drop(SqliteItemRepository::open(&path).unwrap());
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        drop(conn);
        let err = SqliteItemRepository::open(&path).err().unwrap();
        assert!(matches!(err, StorageError::NewerSchema { found: 99, .. }));
    }

    #[test]
    fn corrupt_row_is_reported_not_panicking() {
        let repo = SqliteItemRepository::open_in_memory().unwrap();
        repo.conn
            .execute(
                "INSERT INTO items (id, name, created_at, updated_at) VALUES ('not-a-uuid', 'x', 0, 0)",
                [],
            )
            .unwrap();
        assert!(matches!(repo.list(), Err(StorageError::Corrupt(_))));
    }
}
