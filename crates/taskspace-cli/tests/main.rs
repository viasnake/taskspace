use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use tempfile::tempdir;

#[test]
fn binary_start_and_list_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut start = Command::cargo_bin("taskspace").expect("binary");
    start
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("start")
        .arg("demo task")
        .assert()
        .success()
        .stdout(contains("started task:"));

    let mut list = Command::cargo_bin("taskspace").expect("binary");
    list.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("list")
        .assert()
        .success()
        .stdout(contains("demo task"));
}

#[test]
fn binary_attach_and_detach_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    let source = temp.path().join("source");
    fs::create_dir_all(&source).expect("mkdir");

    let mut start = Command::cargo_bin("taskspace").expect("binary");
    let output = start
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("start")
        .arg("attach task")
        .output()
        .expect("start output");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let id = text.split_whitespace().nth(2).expect("task id").to_string();

    let mut attach = Command::cargo_bin("taskspace").expect("binary");
    let attach_out = attach
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("attach")
        .arg(&id)
        .arg(source.to_str().expect("utf8"))
        .arg("--type")
        .arg("dir")
        .arg("--role")
        .arg("source")
        .arg("--rw")
        .output()
        .expect("attach output");
    assert!(attach_out.status.success());
    let attach_text = String::from_utf8_lossy(&attach_out.stdout);
    let root_id = attach_text
        .split_whitespace()
        .last()
        .expect("root id")
        .trim()
        .to_string();

    let mut detach = Command::cargo_bin("taskspace").expect("binary");
    detach
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("detach")
        .arg(&id)
        .arg(&root_id)
        .assert()
        .success()
        .stdout(contains("detached root"));
}

#[test]
fn binary_finish_archive_and_gc_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut start = Command::cargo_bin("taskspace").expect("binary");
    let output = start
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("start")
        .arg("done task")
        .output()
        .expect("start output");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    let id = text.split_whitespace().nth(2).expect("task id").to_string();

    let mut finish = Command::cargo_bin("taskspace").expect("binary");
    finish
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("finish")
        .arg(&id)
        .assert()
        .success()
        .stdout(contains("task state updated: done"));

    let mut archive = Command::cargo_bin("taskspace").expect("binary");
    archive
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("archive")
        .arg(&id)
        .assert()
        .success()
        .stdout(contains("task archived"));

    let mut gc = Command::cargo_bin("taskspace").expect("binary");
    gc.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("gc")
        .assert()
        .success();
}

#[test]
fn binary_completion_outputs_script() {
    let mut completion = Command::cargo_bin("taskspace").expect("binary");
    completion
        .arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(contains("__complete-tasks"));
}

#[test]
fn binary_complete_tasks_lists_task_ids() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut start = Command::cargo_bin("taskspace").expect("binary");
    start
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("start")
        .arg("completion task")
        .assert()
        .success();

    let mut complete = Command::cargo_bin("taskspace").expect("binary");
    complete
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("__complete-tasks")
        .assert()
        .success()
        .stdout(contains("tsk_"));
}
