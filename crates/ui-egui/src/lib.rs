//! Reference user interface in egui.
//!
//! [`Shell`] draws the whole application. It talks to the core **only** through
//! [`App::execute`](starter_application::App::execute) and the events it emits; it never touches a
//! repository, a thread or a platform API. That is what keeps the UI swappable: another frontend
//! (Dioxus, Iced, Slint, Tauri) drives the same `App` with the same commands.
//!
//! The crate depends on `egui` only (not `eframe`), so hosts decide about windowing and rendering.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

mod shell;
mod views;

pub use shell::Shell;
