//! Text and JSON rendering. JSON output is an envelope:
//! `{"ok": true, "result": …}` or `{"ok": false, "error": {"code": …, "message": …}}`.

use std::io::Write as _;

use serde::Serialize;
use serde_json::json;
use starter_application::{CommandInfo, Output, domain::Item, jobs::JobOutput};

pub struct Printer {
    pub json: bool,
}

impl Printer {
    pub fn output(&self, output: &Output) {
        if self.json {
            return self.ok(output);
        }
        match output {
            Output::Item(item) => print_item(item),
            Output::Items(items) if items.is_empty() => println!("No items."),
            Output::Items(items) => {
                for item in items {
                    println!("{}  {}  {}", item.id, item.created_at, item.name);
                }
            }
            Output::Deleted { id } => println!("Deleted {id}"),
            Output::Info(info) => {
                println!("Version:      {}", env!("CARGO_PKG_VERSION"));
                println!("Storage:      {}", info.storage);
                println!("Items:        {}", info.item_count);
                println!("Running jobs: {}", info.running_jobs);
            }
            Output::JobStarted { id } => println!("Started job {id}"),
            Output::CancelRequested { id } => println!("Cancellation of job {id} requested"),
            Output::Jobs(jobs) => {
                for job in jobs {
                    println!("{}  {}  {:?}", job.id, job.label, job.state);
                }
            }
        }
    }

    pub fn commands(&self, commands: &[CommandInfo]) {
        if self.json {
            return self.ok(commands);
        }
        for info in commands {
            let effect = format!("{:?}", info.effect).to_lowercase();
            println!("{:<12} {:<12} {}", info.name, effect, info.description);
        }
    }

    pub fn job_output(&self, output: &JobOutput) {
        if self.json {
            return self.ok(output);
        }
        match output {
            JobOutput::Report {
                item_count,
                total_characters,
                longest_name,
            } => {
                println!("Items:            {item_count}");
                println!("Total characters: {total_characters}");
                println!("Longest name:     {}", longest_name.as_deref().unwrap_or("-"));
            }
        }
    }

    /// Progress goes to stderr so stdout stays parseable.
    pub fn progress(&self, done: u32, total: u32) {
        if !self.json {
            eprint!("\rProgress {done}/{total}");
            if done == total {
                eprintln!();
            }
            let _ = std::io::stderr().flush();
        }
    }

    pub fn raw_json(&self, value: &serde_json::Value) {
        println!("{}", serde_json::to_string_pretty(value).unwrap_or_default());
    }

    pub fn error(&self, code: &str, message: &str) {
        if self.json {
            println!(
                "{}",
                json!({ "ok": false, "error": { "code": code, "message": message } })
            );
        } else {
            eprintln!("error ({code}): {message}");
        }
    }

    fn ok(&self, result: &(impl Serialize + ?Sized)) {
        println!("{}", json!({ "ok": true, "result": result }));
    }
}

fn print_item(item: &Item) {
    println!("Id:      {}", item.id);
    println!("Name:    {}", item.name);
    println!("Created: {}", item.created_at);
    println!("Updated: {}", item.updated_at);
}
