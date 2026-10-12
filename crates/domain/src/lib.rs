//! Domain model of the reference application: an item manager.
//!
//! This crate is pure Rust. It has no I/O, no UI, no database and no platform code, so it compiles for
//! every target (native, `wasm32-unknown-unknown`) and is tested with plain unit tests.
//!
//! Replace [`Item`] with the entities of your own application; keep the rules of this crate.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

mod error;
mod item;
mod time;

pub use error::DomainError;
pub use item::{Item, ItemId, ItemName};
pub use time::Timestamp;
