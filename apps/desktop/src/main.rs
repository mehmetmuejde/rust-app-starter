//! Native desktop host: wires SQLite storage, the thread job runner and the egui shell into eframe.
//!
//! This file is the composition root – the only place that knows which adapters are used.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]
// No console window on Windows release builds.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::{path::PathBuf, sync::Arc};

use anyhow::Context as _;
use clap::Parser;
use eframe::egui;
use starter_application::{App, ItemRepository, SystemClock};
use starter_platform::native::{ThreadJobRunner, default_data_dir, init_logging};
use starter_storage_sqlite::SqliteItemRepository;
use starter_ui_egui::Shell;

const APP_NAME: &str = "rust-app";

/// Desktop application of the Rust App Starter.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Directory for the database. Defaults to the per-user data directory.
    #[arg(long, env = "RUST_APP_DATA_DIR")]
    data_dir: Option<PathBuf>,
    /// Keep all data in memory (nothing is written to disk).
    #[arg(long)]
    in_memory: bool,
}

struct DesktopApp {
    shell: Shell,
    app: App,
}

impl eframe::App for DesktopApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.shell.logic(ctx, &mut self.app);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.app);
    }
}

fn open_repository(args: &Args) -> anyhow::Result<Box<dyn ItemRepository>> {
    if args.in_memory {
        return Ok(Box::new(SqliteItemRepository::open_in_memory()?));
    }
    let dir = args
        .data_dir
        .clone()
        .or_else(|| default_data_dir(APP_NAME))
        .context("no data directory: pass --data-dir")?;
    let path = SqliteItemRepository::default_path(&dir);
    let repo = SqliteItemRepository::open(&path).with_context(|| format!("opening {}", path.display()))?;
    Ok(Box::new(repo))
}

fn main() -> anyhow::Result<()> {
    init_logging();
    let args = Args::parse();
    // Open storage before the window, so start-up errors reach the terminal instead of a blank window.
    let repo = open_repository(&args)?;
    log::info!("storage: {}", repo.describe());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rust App Starter")
            .with_inner_size([960.0, 640.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rust App Starter",
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            let jobs = ThreadJobRunner::new(
                ThreadJobRunner::default_threads(),
                Some(Arc::new(move || ctx.request_repaint())),
            );
            let app = App::new(repo, Box::new(jobs), Box::new(SystemClock));
            let platform = format!("Desktop ({})", std::env::consts::OS);
            Ok(Box::new(DesktopApp {
                shell: Shell::new(platform),
                app,
            }))
        }),
    )
    .map_err(|err| anyhow::anyhow!("window could not be started: {err}"))
}
