//! Smoke tests: run the real binary against a temporary data directory.

use assert_cmd::Command;
use serde_json::Value;

fn cli(dir: &tempfile::TempDir) -> Command {
    let mut cmd = Command::cargo_bin("rust-app").unwrap();
    cmd.env("RUST_APP_DATA_DIR", dir.path()).env("RUST_LOG", "warn");
    cmd
}

fn json(cmd: &mut Command) -> Value {
    let out = cmd.arg("--json").output().unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn create_list_delete_roundtrip() {
    let dir = tempfile::tempdir().unwrap();

    let created = json(cli(&dir).args(["create", "Groceries"]));
    assert_eq!(created["ok"], true);
    let id = created["result"]["data"]["id"].as_str().unwrap().to_owned();

    let listed = json(cli(&dir).arg("list"));
    assert_eq!(listed["result"]["data"][0]["name"], "Groceries");

    cli(&dir).args(["delete", &id]).assert().success();
    let listed = json(cli(&dir).arg("list"));
    assert_eq!(listed["result"]["data"].as_array().unwrap().len(), 0);
}

#[test]
fn validation_error_exits_with_1() {
    let dir = tempfile::tempdir().unwrap();
    let out = cli(&dir).args(["--json", "create", "   "]).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let body: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(body["error"]["code"], "validation");
}

#[test]
fn unknown_id_exits_with_1() {
    let dir = tempfile::tempdir().unwrap();
    cli(&dir)
        .args(["get", "0190a000-0000-7000-8000-000000000000"])
        .assert()
        .code(1);
}

#[test]
fn usage_error_exits_with_2() {
    let dir = tempfile::tempdir().unwrap();
    cli(&dir).arg("frobnicate").assert().code(2);
}

#[test]
fn exec_runs_json_commands_and_guards_destructive_ones() {
    let dir = tempfile::tempdir().unwrap();
    let created = json(cli(&dir).args(["exec", r#"{"command":"create_item","params":{"name":"via exec"}}"#]));
    let id = created["result"]["data"]["id"].as_str().unwrap().to_owned();

    let delete = format!(r#"{{"command":"delete_item","params":{{"id":"{id}"}}}}"#);
    let refused = json(cli(&dir).args(["exec", &delete]));
    assert_eq!(refused["error"]["code"], "destructive_refused");

    let deleted = json(cli(&dir).args(["exec", &delete, "--allow-destructive"]));
    assert_eq!(deleted["ok"], true);
}

#[test]
fn report_runs_job_to_completion() {
    let dir = tempfile::tempdir().unwrap();
    cli(&dir).args(["create", "abc"]).assert().success();
    let report = json(cli(&dir).arg("report"));
    assert_eq!(report["result"]["item_count"], 1);
    assert_eq!(report["result"]["total_characters"], 3);
}

#[test]
fn schema_lists_every_command() {
    let dir = tempfile::tempdir().unwrap();
    let out = cli(&dir).args(["commands", "--schema"]).output().unwrap();
    let schema: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(schema["oneOf"].as_array().unwrap().len(), 9);
}
