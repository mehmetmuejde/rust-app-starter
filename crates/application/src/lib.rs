//! Application layer: the single entry point every frontend uses.
//!
//! - [`Command`] is the serialisable contract for GUI, CLI, tests and (later) MCP or REST.
//! - [`App::execute`] runs a command against the injected ports and returns an [`Output`].
//! - [`AppEvent`]s tell frontends what changed; they pull them with [`App::drain_events`].
//! - Ports ([`ItemRepository`], [`Clock`], [`JobRunner`]) are implemented by adapter crates.
//!
//! The crate is synchronous and single-owner by design (see `docs/adr/0001-*`): no async runtime,
//! no threads, no UI or database dependencies. It compiles for native and `wasm32-unknown-unknown`.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

mod app;
mod command;
mod error;
mod event;
pub mod jobs;
mod memory;
mod ports;

#[cfg(any(test, feature = "contract-tests"))]
pub mod contract;

pub use app::{App, AppInfo};
pub use command::{COMMANDS, Command, CommandInfo, Effect, Output};
pub use error::AppError;
pub use event::AppEvent;
pub use memory::InMemoryItemRepository;
pub use ports::{Clock, ItemRepository, ManualClock, StorageError, SystemClock};

/// Re-exported so frontends need only one dependency for the common types.
pub use starter_domain as domain;
