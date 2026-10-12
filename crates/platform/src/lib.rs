//! Host services that differ per platform. Hosts (apps) pick what they need; the application core
//! never depends on this crate.
//!
//! | Service             | Native ([`native`])          | Browser ([`web`])                    |
//! |---------------------|------------------------------|--------------------------------------|
//! | Job runner          | [`native::ThreadJobRunner`]  | `starter_application::jobs::InlineJobRunner` |
//! | Data location       | [`native::default_data_dir`] | IndexedDB (see `starter-storage-browser`) |
//! | Logging             | [`native::init_logging`]     | [`web::init_logging`] (browser console) |
//! | Durable storage     | file system                  | [`web::request_persistent_storage`]  |
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[cfg(target_arch = "wasm32")]
pub mod web;
