use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;

#[test]
fn binary_new_and_list_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut new_cmd = Command::cargo_bin("taskspace").expect("binary");
    new_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("demo task")
        .assert()
        .success()
        .stdout(contains("tsk_"));

    let mut list = Command::cargo_bin("taskspace").expect("binary");
    list.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("list")
        .assert()
        .success()
        .stdout(contains("demo task"));
}

#[test]
fn binary_repos_and_use_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    fs::create_dir_all(root.join("repos").join("app")).expect("app");
    fs::create_dir_all(root.join("repos").join("infra")).expect("infra");

    let mut new_cmd = Command::cargo_bin("taskspace").expect("binary");
    let output = new_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("auth migration")
        .output()
        .expect("new output");
    assert!(output.status.success());
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let mut repos = Command::cargo_bin("taskspace").expect("binary");
    repos
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("repos")
        .assert()
        .success()
        .stdout(contains("app"));

    let mut use_cmd = Command::cargo_bin("taskspace").expect("binary");
    use_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("use")
        .arg(&id)
        .arg("app")
        .arg("infra")
        .assert()
        .success()
        .stdout(contains("visible_repos: [app, infra]"));
}

#[test]
fn binary_finish_and_gc_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut new_cmd = Command::cargo_bin("taskspace").expect("binary");
    let output = new_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("done task")
        .output()
        .expect("new output");
    assert!(output.status.success());
    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let mut finish = Command::cargo_bin("taskspace").expect("binary");
    finish
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("finish")
        .arg(&id)
        .assert()
        .success()
        .stdout(contains("task state updated: done"));

    let mut gc = Command::cargo_bin("taskspace").expect("binary");
    gc.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("gc")
        .assert()
        .success();
}
