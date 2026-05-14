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
fn binary_init_and_list_work() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("source");
    init_git_repo(&source);
    let root = temp.path().join("taskspace");

    let mut init = Command::cargo_bin("taskspace").expect("binary");
    init.arg("--root")
        .arg(&root)
        .arg("init")
        .arg(&source)
        .arg("--slots")
        .arg("2")
        .assert()
        .success()
        .stdout(contains("agent-1"))
        .stdout(contains("agent-2"));

    let mut list = Command::cargo_bin("taskspace").expect("binary");
    list.arg("--root")
        .arg(&root)
        .arg("list")
        .assert()
        .success()
        .stdout(contains("agent-1"));
}

#[test]
fn binary_show_checkout_and_hook_context_work() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("source");
    init_git_repo(&source);
    let root = temp.path().join("taskspace");

    let mut init = Command::cargo_bin("taskspace").expect("binary");
    init.arg("--root")
        .arg(&root)
        .arg("init")
        .arg(&source)
        .arg("--slots")
        .arg("1")
        .assert()
        .success();

    let mut show = Command::cargo_bin("taskspace").expect("binary");
    show.arg("--root")
        .arg(&root)
        .arg("show")
        .arg("agent-1")
        .assert()
        .success()
        .stdout(contains("id: agent-1"));

    let mut checkout = Command::cargo_bin("taskspace").expect("binary");
    checkout
        .arg("--root")
        .arg(&root)
        .arg("checkout")
        .arg("agent-1")
        .arg("HEAD")
        .assert()
        .success()
        .stdout(contains("checked out HEAD"));

    let slot_path = root.join("workspaces").join("agent-1");
    let mut context = Command::cargo_bin("taskspace").expect("binary");
    context
        .arg("--root")
        .arg(&root)
        .arg("hook-context")
        .arg(&slot_path)
        .assert()
        .success()
        .stdout(contains("schema_version: 1"))
        .stdout(contains("agent-1"));
}
