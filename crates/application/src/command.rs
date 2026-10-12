use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use starter_domain::{Item, ItemId};

use crate::{
    AppInfo,
    jobs::{JobId, JobInfo, JobRequest},
};

/// Everything a frontend can ask the application to do.
///
/// Serialised as `{"command": "<name>", "params": {...}}`, e.g.
/// `{"command":"create_item","params":{"name":"Groceries"}}`. The JSON Schema of this enum
/// ([`Command::json_schema`]) is the machine-readable contract for agents and tooling.
///
/// To add a command: add a variant, add its [`CommandInfo`] to [`COMMANDS`], handle it in
/// `App::execute` (the compiler enforces the last step; a test enforces the second).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "command", content = "params", rename_all = "snake_case")]
pub enum Command {
    /// Create a new item.
    CreateItem { name: String },
    /// Rename an existing item.
    RenameItem { id: ItemId, name: String },
    /// Delete an item permanently.
    DeleteItem { id: ItemId },
    /// List all items, oldest first.
    ListItems,
    /// Get one item by id.
    GetItem { id: ItemId },
    /// Storage location and counters.
    Info,
    /// Start a background job.
    StartJob { job: JobRequest },
    /// Ask a running job to stop.
    CancelJob { id: JobId },
    /// List known jobs (running and recently finished).
    ListJobs,
}

/// What a command does to the application's data. Drives confirmation prompts and agent tool hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Reads only.
    Read,
    /// Changes data in a recoverable way.
    Write,
    /// Removes data. Frontends should ask before running it on behalf of someone else (e.g. an agent).
    Destructive,
}

/// Static description of a command for menus, `--help`, tool discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CommandInfo {
    pub name: &'static str,
    pub label: &'static str,
    pub effect: Effect,
    pub description: &'static str,
}

/// Catalogue of all commands. Kept in sync with [`Command`] by a test.
pub const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        name: "create_item",
        label: "Create item",
        effect: Effect::Write,
        description: "Create a new item.",
    },
    CommandInfo {
        name: "rename_item",
        label: "Rename item",
        effect: Effect::Write,
        description: "Rename an existing item.",
    },
    CommandInfo {
        name: "delete_item",
        label: "Delete item",
        effect: Effect::Destructive,
        description: "Delete an item permanently.",
    },
    CommandInfo {
        name: "list_items",
        label: "List items",
        effect: Effect::Read,
        description: "List all items.",
    },
    CommandInfo {
        name: "get_item",
        label: "Get item",
        effect: Effect::Read,
        description: "Get one item by id.",
    },
    CommandInfo {
        name: "info",
        label: "Info",
        effect: Effect::Read,
        description: "Storage location and counters.",
    },
    CommandInfo {
        name: "start_job",
        label: "Start job",
        effect: Effect::Write,
        description: "Start a background job.",
    },
    CommandInfo {
        name: "cancel_job",
        label: "Cancel job",
        effect: Effect::Write,
        description: "Ask a running job to stop.",
    },
    CommandInfo {
        name: "list_jobs",
        label: "List jobs",
        effect: Effect::Read,
        description: "List known jobs.",
    },
];

impl Command {
    /// The serialised name (`command` tag).
    pub fn name(&self) -> &'static str {
        match self {
            Self::CreateItem { .. } => "create_item",
            Self::RenameItem { .. } => "rename_item",
            Self::DeleteItem { .. } => "delete_item",
            Self::ListItems => "list_items",
            Self::GetItem { .. } => "get_item",
            Self::Info => "info",
            Self::StartJob { .. } => "start_job",
            Self::CancelJob { .. } => "cancel_job",
            Self::ListJobs => "list_jobs",
        }
    }

    pub fn info(&self) -> Option<&'static CommandInfo> {
        let name = self.name();
        COMMANDS.iter().find(|info| info.name == name)
    }

    /// Effect of this command. Unknown metadata is treated as destructive (fail safe).
    pub fn effect(&self) -> Effect {
        self.info().map_or(Effect::Destructive, |info| info.effect)
    }

    /// JSON Schema of the whole command enum.
    pub fn json_schema() -> schemars::Schema {
        schemars::schema_for!(Command)
    }
}

/// Result of a successful command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Output {
    Item(Item),
    Items(Vec<Item>),
    Deleted { id: ItemId },
    Info(AppInfo),
    JobStarted { id: JobId },
    CancelRequested { id: JobId },
    Jobs(Vec<JobInfo>),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn schema_command_names() -> BTreeSet<String> {
        let schema = serde_json::to_value(Command::json_schema()).unwrap();
        let variants = schema["oneOf"]
            .as_array()
            .expect("adjacently tagged enum renders as oneOf");
        variants
            .iter()
            .map(|v| {
                let tag = &v["properties"]["command"];
                tag["const"]
                    .as_str()
                    .or_else(|| tag["enum"][0].as_str())
                    .unwrap()
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn catalogue_matches_enum() {
        let catalogue: BTreeSet<String> = COMMANDS.iter().map(|c| c.name.to_owned()).collect();
        assert_eq!(
            catalogue,
            schema_command_names(),
            "COMMANDS and Command variants differ"
        );
    }

    #[test]
    fn parses_documented_json_shape() {
        let cmd: Command = serde_json::from_str(r#"{"command":"create_item","params":{"name":"Groceries"}}"#).unwrap();
        assert_eq!(
            cmd,
            Command::CreateItem {
                name: "Groceries".into()
            }
        );
        let cmd: Command = serde_json::from_str(r#"{"command":"list_items"}"#).unwrap();
        assert_eq!(cmd, Command::ListItems);
    }

    #[test]
    fn unknown_command_is_rejected() {
        assert!(serde_json::from_str::<Command>(r#"{"command":"drop_database"}"#).is_err());
    }

    #[test]
    fn delete_is_destructive() {
        let cmd = Command::DeleteItem { id: ItemId::generate() };
        assert_eq!(cmd.effect(), Effect::Destructive);
    }
}
