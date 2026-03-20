use assert_cmd::Command;
use tempfile::tempdir;

#[test]
fn binary_new_and_list_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("sessions");

    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("demo")
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("list")
        .assert()
        .success();
}

#[test]
fn binary_rm_requires_yes() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().join("sessions");

    let mut new_cmd = Command::cargo_bin("taskspace").expect("binary");
    new_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("demo")
        .assert()
        .success();

    let mut rm_cmd = Command::cargo_bin("taskspace").expect("binary");
    rm_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("rm")
        .arg("demo")
        .assert()
        .failure();
}
