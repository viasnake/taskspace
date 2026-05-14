use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

fn init_git_repo(path: &Path) {
    fs::create_dir_all(path).expect("repo dir");

    let mut init = Command::new("git");
    init.arg("init").arg(path).assert().success();

    fs::write(path.join("README.md"), "demo\n").expect("readme");

    let mut add = Command::new("git");
    add.arg("-C")
        .arg(path)
        .arg("add")
        .arg("README.md")
        .assert()
        .success();

    let mut commit = Command::new("git");
    commit
        .arg("-C")
        .arg(path)
        .arg("-c")
        .arg("user.name=Test")
        .arg("-c")
        .arg("user.email=test@example.com")
        .arg("commit")
        .arg("-m")
        .arg("initial")
        .assert()
        .success();
}

#[test]
fn binary_project_and_slot_commands_work() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("source");
    init_git_repo(&source);
    let root = temp.path().join("taskspace");

    let mut init = Command::cargo_bin("taskspace").expect("binary");
    init.arg("--root")
        .arg(&root)
        .arg("init")
        .assert()
        .success()
        .stdout(contains("initialized taskspace"));

    let mut project_add = Command::cargo_bin("taskspace").expect("binary");
    project_add
        .arg("--root")
        .arg(&root)
        .arg("project")
        .arg("add")
        .arg("app")
        .arg(&source)
        .assert()
        .success()
        .stdout(contains("registered project app"));

    let mut slot_add = Command::cargo_bin("taskspace").expect("binary");
    slot_add
        .arg("--root")
        .arg(&root)
        .arg("slot")
        .arg("add")
        .arg("app")
        .arg("--count")
        .arg("2")
        .assert()
        .success()
        .stdout(contains("app:agent-1"))
        .stdout(contains("app:agent-2"));

    let mut list = Command::cargo_bin("taskspace").expect("binary");
    list.arg("--root")
        .arg(&root)
        .arg("slot")
        .arg("list")
        .assert()
        .success()
        .stdout(contains("app:agent-1"));
}

#[test]
fn binary_show_sync_and_hook_context_work() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("source");
    init_git_repo(&source);
    let root = temp.path().join("taskspace");

    let mut init = Command::cargo_bin("taskspace").expect("binary");
    init.arg("--root")
        .arg(&root)
        .arg("init")
        .assert()
        .success();

    let mut project_add = Command::cargo_bin("taskspace").expect("binary");
    project_add
        .arg("--root")
        .arg(&root)
        .arg("project")
        .arg("add")
        .arg("app")
        .arg(&source)
        .assert()
        .success();

    let mut slot_add = Command::cargo_bin("taskspace").expect("binary");
    slot_add
        .arg("--root")
        .arg(&root)
        .arg("slot")
        .arg("add")
        .arg("app")
        .assert()
        .success();

    let mut show = Command::cargo_bin("taskspace").expect("binary");
    show.arg("--root")
        .arg(&root)
        .arg("slot")
        .arg("show")
        .arg("app:agent-1")
        .assert()
        .success()
        .stdout(contains("ref: app:agent-1"));

    let mut sync = Command::cargo_bin("taskspace").expect("binary");
    sync.arg("--root")
        .arg(&root)
        .arg("sync")
        .arg("app")
        .assert()
        .success()
        .stdout(contains("ok\tapp:agent-1\tfetched"));

    let slot_path = root.join("workspaces").join("app").join("agent-1");
    let mut context = Command::cargo_bin("taskspace").expect("binary");
    context
        .arg("--root")
        .arg(&root)
        .arg("hook-context")
        .arg(&slot_path)
        .assert()
        .success()
        .stdout(contains("schema_version: 2"))
        .stdout(contains("project:"))
        .stdout(contains("id: app"));
}
