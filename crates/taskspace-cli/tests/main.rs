use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn binary_new_and_list_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

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
fn binary_new_with_unknown_editor_fails_fast() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("demo")
        .arg("--editor")
        .arg("definitely-not-installed-editor-xyz")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown editor"));

    assert!(!root.join("demo").exists());
}

#[test]
fn binary_rm_with_yes_succeeds() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

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
        .arg("--yes")
        .assert()
        .success()
        .stdout(predicates::str::contains("removed session: demo"));
}

#[test]
fn binary_rm_without_yes_fails_even_with_stdin_input_in_non_interactive_mode() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

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
        .write_stdin("y\n")
        .assert()
        .failure()
        .stderr(predicates::str::contains("without --yes"));
}

#[test]
fn binary_rm_dry_run_succeeds_without_yes() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

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
        .arg("--dry-run")
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "dry-run: session 'demo' can be removed",
        ));
}

#[test]
fn binary_version_short_v_works() {
    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.arg("-v")
        .assert()
        .success()
        .stdout(predicates::str::contains("taskspace"));
}

#[test]
fn binary_list_empty_shows_message() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("list")
        .assert()
        .success()
        .stdout("no sessions found\n");
}

#[test]
fn binary_open_without_name_uses_latest_session() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut new_old = Command::cargo_bin("taskspace").expect("binary");
    new_old
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("old")
        .assert()
        .success();

    let mut new_new = Command::cargo_bin("taskspace").expect("binary");
    new_new
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("new")
        .assert()
        .success();

    let old_dir = root.join("old");
    let new_dir = root.join("new");
    let old_file = old_dir.join("SESSION.md");
    let new_file = new_dir.join("SESSION.md");
    fs::write(&old_file, "old").expect("touch old");
    fs::write(&new_file, "new").expect("touch new");

    // Create stub directory OUTSIDE the root to avoid being treated as a session
    let stub_temp = tempdir().expect("stub tempdir");
    let bin_dir = stub_temp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    let marker = root.join("called.txt");
    let stub = bin_dir.join("opencode");
    fs::write(
        &stub,
        format!("#!/bin/sh\nprintf '%s' \"$1\" > '{}'\n", marker.display()),
    )
    .expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&stub).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&stub, perms).expect("chmod");
    }

    let original_path = std::env::var("PATH").unwrap_or_default();
    let path_value = format!("{}:{}", bin_dir.display(), original_path);

    let mut open_cmd = Command::cargo_bin("taskspace").expect("binary");
    open_cmd
        .env("PATH", path_value)
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("open")
        .assert()
        .success();

    let called = fs::read_to_string(&marker).expect("read marker");
    // For Opencode editor, the session directory is passed directly.
    // We verify that "new" session is opened (the latest one)
    assert!(
        called.contains("/new") || called.ends_with("new"),
        "expected /new in path, got: {called}"
    );
}

#[test]
fn binary_aliases_work() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut new_cmd = Command::cargo_bin("taskspace").expect("binary");
    new_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("demo")
        .assert()
        .success();

    let mut ls_cmd = Command::cargo_bin("taskspace").expect("binary");
    ls_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("ls")
        .assert()
        .success()
        .stdout(predicates::str::contains("demo"));

    let mut remove_cmd = Command::cargo_bin("taskspace").expect("binary");
    remove_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("remove")
        .arg("demo")
        .arg("--yes")
        .assert()
        .success();
}

#[test]
fn binary_completion_bash_outputs_script() {
    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.arg("completion")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicates::str::contains("taskspace"))
        .stdout(predicates::str::contains("__complete-sessions"));
}

#[test]
fn binary_completion_without_shell_uses_detected_shell() {
    let mut cmd = Command::cargo_bin("taskspace").expect("binary");
    cmd.env("SHELL", "/bin/bash")
        .arg("completion")
        .assert()
        .success()
        .stdout(predicates::str::contains("taskspace"))
        .stdout(predicates::str::contains("complete -F"));
}

#[test]
fn binary_completion_rejects_powershell_and_elvish() {
    let mut powershell = Command::cargo_bin("taskspace").expect("binary");
    powershell
        .arg("completion")
        .arg("powershell")
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value"));

    let mut elvish = Command::cargo_bin("taskspace").expect("binary");
    elvish
        .arg("completion")
        .arg("elvish")
        .assert()
        .failure()
        .stderr(predicates::str::contains("invalid value"));
}

#[test]
fn binary_complete_sessions_lists_sessions_only() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut new_cmd = Command::cargo_bin("taskspace").expect("binary");
    new_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("new")
        .arg("demo")
        .assert()
        .success();

    let mut complete_cmd = Command::cargo_bin("taskspace").expect("binary");
    complete_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("__complete-sessions")
        .assert()
        .success()
        .stdout("demo\n");
}

#[test]
fn binary_complete_sessions_empty_outputs_nothing() {
    let temp = tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();

    let mut complete_cmd = Command::cargo_bin("taskspace").expect("binary");
    complete_cmd
        .arg("--root")
        .arg(root.to_str().expect("utf8"))
        .arg("__complete-sessions")
        .assert()
        .success()
        .stdout("");
}
