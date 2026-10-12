//! Minimal IndexedDB access on raw `web-sys`: one object store, one key holding the snapshot.

use js_sys::Promise;
use starter_application::StorageError;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransactionMode};

use crate::SnapshotStore;

const STORE: &str = "snapshots";
const KEY: &str = "items";
const DB_VERSION: u32 = 1;

/// Snapshot store in the browser's IndexedDB.
pub struct IndexedDbSnapshotStore {
    db: IdbDatabase,
    name: String,
}

impl IndexedDbSnapshotStore {
    /// Opens (and on first use creates) the database `name`.
    pub async fn open(name: &str) -> Result<Self, StorageError> {
        let window = web_sys::window().ok_or_else(|| backend("no window object"))?;
        let factory = window
            .indexed_db()
            .map_err(js_err("access IndexedDB"))?
            .ok_or_else(|| backend("IndexedDB is not available (private mode or disabled storage?)"))?;
        let request: IdbOpenDbRequest = factory
            .open_with_u32(name, DB_VERSION)
            .map_err(js_err("open database"))?;

        let upgrade_request = request.clone();
        let on_upgrade = Closure::once_into_js(move |_event: web_sys::Event| {
            let Ok(result) = upgrade_request.result() else {
                log::error!("IndexedDB upgrade: no database handle");
                return;
            };
            let db: IdbDatabase = result.unchecked_into();
            if !db.object_store_names().contains(STORE)
                && let Err(err) = db.create_object_store(STORE)
            {
                log::error!("IndexedDB upgrade: create object store failed: {err:?}");
            }
        });
        request.set_onupgradeneeded(Some(on_upgrade.unchecked_ref()));

        let db: IdbDatabase = request_result(&request)
            .await?
            .dyn_into()
            .map_err(js_err("database handle"))?;
        Ok(Self {
            db,
            name: name.to_owned(),
        })
    }

    /// Reads the stored snapshot, `None` on first start.
    pub async fn load(&self) -> Result<Option<String>, StorageError> {
        let store = self
            .db
            .transaction_with_str(STORE)
            .and_then(|tx| tx.object_store(STORE))
            .map_err(js_err("open read transaction"))?;
        let request = store.get(&JsValue::from_str(KEY)).map_err(js_err("read snapshot"))?;
        Ok(request_result(&request).await?.as_string())
    }
}

impl SnapshotStore for IndexedDbSnapshotStore {
    fn save(&mut self, snapshot_json: String) -> Result<(), StorageError> {
        let store = self
            .db
            .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)
            .and_then(|tx| tx.object_store(STORE))
            .map_err(js_err("open write transaction"))?;
        let request = store
            .put_with_key(&JsValue::from_str(&snapshot_json), &JsValue::from_str(KEY))
            .map_err(js_err("write snapshot"))?;
        // Completion is asynchronous; IndexedDB applies readwrite transactions in creation order.
        let on_error = Closure::once_into_js(|event: web_sys::Event| {
            log::error!("IndexedDB snapshot write failed: {:?}", event.type_());
        });
        request.set_onerror(Some(on_error.unchecked_ref()));
        Ok(())
    }

    fn describe(&self) -> String {
        format!("IndexedDB: database '{}'", self.name)
    }
}

/// Turns an `IDBRequest` into a future resolving to its result.
async fn request_result(request: &IdbRequest) -> Result<JsValue, StorageError> {
    let promise = Promise::new(&mut |resolve, reject| {
        let success_request = request.clone();
        let on_success = Closure::once_into_js(move |_event: web_sys::Event| {
            let value = success_request.result().unwrap_or(JsValue::UNDEFINED);
            if let Err(err) = resolve.call1(&JsValue::NULL, &value) {
                log::error!("IndexedDB: resolve failed: {err:?}");
            }
        });
        let error_request = request.clone();
        let on_error = Closure::once_into_js(move |_event: web_sys::Event| {
            let message = error_request
                .error()
                .ok()
                .flatten()
                .map_or_else(|| "unknown IndexedDB error".to_owned(), |e| e.message());
            if let Err(err) = reject.call1(&JsValue::NULL, &JsValue::from_str(&message)) {
                log::error!("IndexedDB: reject failed: {err:?}");
            }
        });
        request.set_onsuccess(Some(on_success.unchecked_ref()));
        request.set_onerror(Some(on_error.unchecked_ref()));
    });
    JsFuture::from(promise).await.map_err(js_err("IndexedDB request"))
}

fn backend(message: &str) -> StorageError {
    StorageError::Backend(message.to_owned())
}

fn js_err(context: &'static str) -> impl Fn(JsValue) -> StorageError {
    move |value| {
        StorageError::Backend(format!(
            "{context}: {}",
            value.as_string().unwrap_or_else(|| format!("{value:?}"))
        ))
    }
}
