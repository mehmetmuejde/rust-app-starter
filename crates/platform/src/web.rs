//! Browser host services: console logging and the persistent-storage request.

use log::{Level, LevelFilter, Log, Metadata, Record};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

struct ConsoleLogger;

impl Log for ConsoleLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = JsValue::from_str(&format!("[{}] {}: {}", record.level(), record.target(), record.args()));
        match record.level() {
            Level::Error => web_sys::console::error_1(&line),
            Level::Warn => web_sys::console::warn_1(&line),
            Level::Info => web_sys::console::info_1(&line),
            Level::Debug | Level::Trace => web_sys::console::debug_1(&line),
        }
    }

    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;

/// Routes `log` records to the browser console. Safe to call more than once.
pub fn init_logging(level: LevelFilter) {
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(level);
    }
}

/// Asks the browser not to evict this origin's storage under pressure.
/// Returns whether the browser granted it; browsers may decide silently.
pub async fn request_persistent_storage() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(promise) = window.navigator().storage().persist() else {
        return false;
    };
    JsFuture::from(promise)
        .await
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}
