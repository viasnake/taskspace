mod fs;
mod process;

pub use fs::*;
pub use process::*;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn directory_and_file_operations_work() {
        let temp = tempdir().expect("tempdir");
        let dir = temp.path().join("a").join("b");
        create_dir(&dir).expect("create dir");

        let file = dir.join("x.txt");
        write_file(&file, "hello").expect("write file");
        let content = read_file(&file).expect("read file");
        assert_eq!(content, "hello");

        let dirs = list_directories(temp.path()).expect("list dirs");
        assert_eq!(dirs, vec!["a".to_string()]);

        let canonical = canonicalize_if_exists(file.to_str().expect("utf8 path"));
        assert!(canonical.is_some());

        let moved = temp.path().join("moved");
        move_dir(&dir, &moved).expect("move dir");
        assert!(moved.exists());

        remove_dir_all(&moved).expect("remove dir");
        assert!(!moved.exists());
    }

    #[test]
    fn command_execution_paths_are_covered() {
        let temp = tempdir().expect("tempdir");
        let ok_status = run_command("true", &[]);
        assert!(ok_status.is_ok());

        let ok_status_in_dir = run_command_in_dir("true", &[], temp.path());
        assert!(ok_status_in_dir.is_ok());

        let fail_status = run_command("false", &[]);
        assert!(fail_status.is_err());

        let captured = run_command_capture("sh", &["-c".to_string(), "printf hi".to_string()]);
        assert_eq!(captured.expect("capture output"), "hi");

        let captured_fail = run_command_capture("false", &[]);
        assert!(captured_fail.is_err());
    }
}
