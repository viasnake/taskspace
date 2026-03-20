use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use taskspace_app::TaskspaceApp;
use taskspace_core::TaskspaceError;

mod execute;
mod exit_code;
mod parse;
mod render;

#[derive(Parser)]
#[command(name = "taskspace")]
#[command(version, about = "Session-oriented workspace manager for AI coding")]
struct Cli {
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    New {
        name: String,
        #[arg(long = "repo")]
        repos: Vec<String>,
        #[arg(long)]
        open: bool,
        #[arg(long, value_enum, default_value_t = CliEditor::Opencode)]
        editor: CliEditor,
    },
    Open {
        name: String,
        #[arg(long, value_enum, default_value_t = CliEditor::Opencode)]
        editor: CliEditor,
    },
    List,
    Rm {
        name: String,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
    Archive {
        name: String,
    },
    Doctor,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CliEditor {
    Opencode,
    Code,
}

#[cfg(not(test))]
fn main() {
    match run_with_args(std::env::args_os()) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(exit_code::from_error(&err));
        }
    }
}

fn run_with_args<I, T>(args: I) -> Result<Vec<String>, TaskspaceError>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(|err| TaskspaceError::Usage(err.to_string()))?;
    let app = TaskspaceApp::new(cli.root).map_err(execute::map_anyhow_error)?;
    let request = parse::parse_command(cli.command)?;
    let result = execute::execute(&app, request)?;
    Ok(render::render(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn run_new_list_archive_rm_doctor_flow() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("sessions");

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "new",
            "demo",
        ])
        .expect("new should succeed");
        assert!(out[0].contains("created session"));

        let out = run_with_args(["taskspace", "--root", root.to_str().expect("utf8"), "list"])
            .expect("list should succeed");
        assert_eq!(out, vec!["demo".to_string()]);

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "archive",
            "demo",
        ])
        .expect("archive should succeed");
        assert!(out[0].contains("archived session to"));

        run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "new",
            "demo2",
        ])
        .expect("new demo2 should succeed");

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "rm",
            "demo2",
            "--dry-run",
        ])
        .expect("dry run should succeed");
        assert!(out[0].contains("dry-run"));

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "rm",
            "demo2",
            "--yes",
        ])
        .expect("remove should succeed");
        assert!(out[0].contains("removed session"));

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "doctor",
        ])
        .expect("doctor should succeed");
        assert!(!out.is_empty());
    }

    #[test]
    fn run_reports_usage_errors() {
        let err = run_with_args(["taskspace", "new"]).expect_err("parse should fail");
        assert!(matches!(err, TaskspaceError::Usage(_)));
        assert_eq!(exit_code::from_error(&err), 2);
    }

    #[test]
    fn doctor_outputs_fail_label() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("sessions");

        run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "new",
            "demo",
        ])
        .expect("create session");

        fs::remove_file(root.join("demo/context/PLAN.md")).expect("remove file");
        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "doctor",
        ])
        .expect("doctor should run");
        assert!(out.iter().any(|line| line.starts_with("[FAIL]")));
    }
}
