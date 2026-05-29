use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::TempDir;

fn miao_in(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("miao").expect("miao binary built");
    cmd.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("APPDATA", home.join("AppData/Roaming"))
        .env("LOCALAPPDATA", home.join("AppData/Local"));
    cmd
}

fn isolated_home() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

#[test]
fn version_flag_prints_version() {
    let home = isolated_home();
    miao_in(home.path())
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("miao"));
}

#[test]
fn help_flag_lists_subcommands() {
    let home = isolated_home();
    miao_in(home.path())
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("launch"))
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("mod-search"));
}

#[test]
fn list_on_empty_data_dir_prints_no_instances() {
    let home = isolated_home();
    miao_in(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No instances found"));
}

#[test]
fn account_command_creates_offline_account() {
    let home = isolated_home();
    miao_in(home.path())
        .args(["account", "TestPlayer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("TestPlayer"));
}

#[test]
fn account_then_list_shows_no_instances_but_account_persists() {
    let home = isolated_home();
    miao_in(home.path())
        .args(["account", "Persisted"])
        .assert()
        .success();
    miao_in(home.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("No instances found"));
}

#[test]
fn unknown_subcommand_exits_with_error() {
    let home = isolated_home();
    miao_in(home.path())
        .arg("definitely-not-a-real-subcommand")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"));
}

#[test]
fn new_without_arguments_fails() {
    let home = isolated_home();
    miao_in(home.path())
        .arg("new")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn launch_unknown_instance_fails_gracefully() {
    let home = isolated_home();
    miao_in(home.path())
        .args(["launch", "ghost-instance-that-does-not-exist"])
        .assert()
        .failure();
}

#[test]
fn mods_command_requires_instance_argument() {
    let home = isolated_home();
    miao_in(home.path())
        .arg("mods")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn delete_unknown_instance_is_idempotent() {
    let home = isolated_home();
    miao_in(home.path())
        .args(["delete", "no-such-instance"])
        .assert()
        .success();
}

#[test]
fn java_subcommand_runs_without_args() {
    let home = isolated_home();
    miao_in(home.path()).arg("java").assert().success();
}

#[test]
fn export_unknown_instance_fails() {
    let home = isolated_home();
    miao_in(home.path())
        .args(["export", "missing"])
        .assert()
        .failure();
}
