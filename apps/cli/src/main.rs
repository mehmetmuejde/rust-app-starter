//! Command-line host. Uses the same `App` and commands as the GUIs; adds nothing but parsing and
//! printing.
//!
//! Exit codes: `0` success · `1` the request was invalid (validation, unknown id, refused command)
//! · `2` usage error (clap) · `3` storage or internal error.
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::unreachable
)]

mod output;

use std::{
    io::Read as _,
    path::PathBuf,
    process::ExitCode,
    time::{Duration, Instant},
};

use clap::{Parser, Subcommand};
use starter_application::{
    App, AppError, AppEvent, COMMANDS, Command, Effect, ItemRepository, Output, SystemClock,
    domain::ItemId,
    jobs::{JobRequest, JobState},
};
use starter_platform::native::{ThreadJobRunner, default_data_dir, init_logging};
use starter_storage_sqlite::SqliteItemRepository;

use crate::output::Printer;

const APP_NAME: &str = "rust-app";

/// Command-line interface of the Rust App Starter.
#[derive(Debug, Parser)]
#[command(name = "rust-app", version, about, propagate_version = true)]
struct Cli {
    /// Directory for the database. Defaults to the per-user data directory.
    #[arg(long, global = true, env = "RUST_APP_DATA_DIR")]
    data_dir: Option<PathBuf>,
    /// Print machine-readable JSON instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(Debug, Subcommand)]
enum CliCommand {
    /// Create an item.
    Create { name: String },
    /// List all items.
    List,
    /// Show one item.
    Get { id: ItemId },
    /// Rename an item.
    Rename { id: ItemId, name: String },
    /// Delete an item.
    Delete { id: ItemId },
    /// Storage location and counters.
    Info,
    /// Run the report job and wait for it.
    Report,
    /// List all application commands (or print their JSON Schema).
    Commands {
        /// Print the JSON Schema of the command format instead of the list.
        #[arg(long)]
        schema: bool,
    },
    /// Execute any command given as JSON, e.g. '{"command":"list_items"}'. Use '-' to read stdin.
    Exec {
        #[arg(value_name = "JSON")]
        command_json: String,
        /// Allow commands whose effect is destructive (e.g. delete_item).
        #[arg(long)]
        allow_destructive: bool,
    },
}

/// Everything that can end a CLI run unsuccessfully.
enum Failure {
    App(AppError),
    Refused { code: &'static str, message: String },
    Internal(String),
}

impl Failure {
    fn exit_code(&self) -> u8 {
        match self {
            Self::App(err) if err.is_user_error() => 1,
            Self::Refused { .. } => 1,
            Self::App(_) | Self::Internal(_) => 3,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::App(err) => err.code(),
            Self::Refused { code, .. } => code,
            Self::Internal(_) => "internal",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::App(err) => err.to_string(),
            Self::Refused { message, .. } | Self::Internal(message) => message.clone(),
        }
    }
}

impl From<AppError> for Failure {
    fn from(err: AppError) -> Self {
        Self::App(err)
    }
}

fn main() -> ExitCode {
    init_logging();
    let cli = Cli::parse();
    let printer = Printer { json: cli.json };
    match run(&cli, &printer) {
        Ok(()) => ExitCode::SUCCESS,
        Err(failure) => {
            printer.error(failure.code(), &failure.message());
            ExitCode::from(failure.exit_code())
        }
    }
}

fn open_app(cli: &Cli) -> Result<App, Failure> {
    let dir = cli
        .data_dir
        .clone()
        .or_else(|| default_data_dir(APP_NAME))
        .ok_or_else(|| Failure::Internal("no data directory: pass --data-dir".into()))?;
    let repo = SqliteItemRepository::open(&SqliteItemRepository::default_path(&dir))
        .map_err(|e| Failure::App(AppError::Storage(e)))?;
    log::debug!("storage: {}", repo.describe());
    Ok(App::new(
        Box::new(repo),
        Box::new(ThreadJobRunner::new(1, None)),
        Box::new(SystemClock),
    ))
}

fn run(cli: &Cli, printer: &Printer) -> Result<(), Failure> {
    // Commands that need no storage.
    if let CliCommand::Commands { schema } = &cli.command {
        if *schema {
            printer.raw_json(&serde_json::to_value(Command::json_schema()).unwrap_or_default());
        } else {
            printer.commands(COMMANDS);
        }
        return Ok(());
    }

    let mut app = open_app(cli)?;
    let command = match &cli.command {
        CliCommand::Create { name } => Command::CreateItem { name: name.clone() },
        CliCommand::List => Command::ListItems,
        CliCommand::Get { id } => Command::GetItem { id: *id },
        CliCommand::Rename { id, name } => Command::RenameItem {
            id: *id,
            name: name.clone(),
        },
        CliCommand::Delete { id } => Command::DeleteItem { id: *id },
        CliCommand::Info => Command::Info,
        CliCommand::Report => return run_report(&mut app, printer),
        CliCommand::Exec {
            command_json,
            allow_destructive,
        } => parse_exec(command_json, *allow_destructive)?,
        CliCommand::Commands { .. } => return Ok(()),
    };
    let output = app.execute(command)?;
    printer.output(&output);
    Ok(())
}

fn parse_exec(json: &str, allow_destructive: bool) -> Result<Command, Failure> {
    let text = if json == "-" {
        let mut buffer = String::new();
        std::io::stdin()
            .read_to_string(&mut buffer)
            .map_err(|e| Failure::Internal(format!("read stdin: {e}")))?;
        buffer
    } else {
        json.to_owned()
    };
    let command: Command = serde_json::from_str(&text).map_err(|e| Failure::Refused {
        code: "invalid_command",
        message: format!("invalid command JSON: {e}"),
    })?;
    if command.effect() == Effect::Destructive && !allow_destructive {
        return Err(Failure::Refused {
            code: "destructive_refused",
            message: format!(
                "'{}' is destructive; pass --allow-destructive to run it",
                command.name()
            ),
        });
    }
    Ok(command)
}

fn run_report(app: &mut App, printer: &Printer) -> Result<(), Failure> {
    let Output::JobStarted { id } = app.execute(Command::StartJob {
        job: JobRequest::GenerateReport { step_delay_ms: 0 },
    })?
    else {
        return Err(Failure::Internal("unexpected output for start_job".into()));
    };
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        app.poll();
        for event in app.drain_events() {
            let AppEvent::JobChanged { job } = event else { continue };
            if job.id != id {
                continue;
            }
            match job.state {
                JobState::Running { done, total } => printer.progress(done, total),
                JobState::Succeeded { output } => {
                    printer.job_output(&output);
                    return Ok(());
                }
                JobState::Failed { error } => return Err(Failure::Internal(format!("job failed: {error}"))),
                JobState::Cancelled => return Err(Failure::Internal("job was cancelled".into())),
                JobState::Queued => {}
            }
        }
        if Instant::now() > deadline {
            return Err(Failure::Internal("job timed out".into()));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
