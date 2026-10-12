//! Browser host: wires the IndexedDB-backed repository, the inline job runner and the egui shell into
//! eframe's `WebRunner`. Build and serve with `cargo xtask web` (uses trunk).
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("starter-web runs in the browser. Use `cargo xtask web` (trunk serve) instead.");
    std::process::exit(2);
}
