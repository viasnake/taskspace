use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{ArgAction, CommandFactory, Parser, Subcommand};
use clap_complete::{Shell, generate};
use taskspace_app::TaskspaceApp;
use taskspace_core::TaskspaceError;

mod execute;
mod exit_code;
mod parse;
mod render;

#[derive(Parser)]
#[command(name = "taskspace")]
#[command(version, about = "Session-oriented workspace manager for AI coding")]
#[command(disable_version_flag = true)]
#[command(propagate_version = true)]
struct Cli {
    #[arg(
        short = 'v',
        visible_short_alias = 'V',
        long = "version",
        action = ArgAction::Version,
        global = true
    )]
    _version: Option<bool>,

    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    New {
        name: String,
        #[arg(long = "template")]
        template: Option<String>,
        #[arg(long)]
        open: bool,
        #[arg(long, default_value = "opencode")]
        editor: String,
    },
    Open {
        name: Option<String>,
        #[arg(long, default_value = "opencode")]
        editor: String,
        #[arg(long)]
        last: bool,
    },
    #[command(alias = "ls")]
    List,
    #[command(alias = "remove")]
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
    Completion {
        #[arg(value_enum)]
        shell: Option<Shell>,
    },
}

#[cfg(not(test))]
fn main() {
    match parse_cli(std::env::args_os()) {
        Ok(cli) => match run_with_cli(cli) {
            Ok(lines) => {
                for line in lines {
                    println!("{line}");
                }
            }
            Err(err) => {
                eprintln!("{err}");
                std::process::exit(exit_code::from_error(&err));
            }
        },
        Err(ParseOutcome::Display(text)) => {
            print!("{text}");
            std::process::exit(0);
        }
        Err(ParseOutcome::Usage(text)) => {
            eprint!("{text}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
fn run_with_args<I, T>(args: I) -> Result<Vec<String>, TaskspaceError>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = parse_cli(args).map_err(|outcome| match outcome {
        ParseOutcome::Display(text) | ParseOutcome::Usage(text) => TaskspaceError::Usage(text),
    })?;
    run_with_cli(cli)
}

fn run_with_cli(cli: Cli) -> Result<Vec<String>, TaskspaceError> {
    let Cli {
        root,
        command,
        _version: _,
    } = cli;
    let request = parse::parse_command(command)?;
    let request = match request {
        execute::CommandRequest::Completion { shell } => {
            let shell = shell.unwrap_or_else(detect_shell);
            return Ok(vec![render_completion(shell)?]);
        }
        other => other,
    };
    let app = TaskspaceApp::new(root).map_err(execute::map_anyhow_error)?;
    let stdin = std::io::stdin();
    let stdin_is_terminal = stdin.is_terminal();
    let mut input = stdin.lock();
    let mut output = std::io::stderr().lock();
    let request = maybe_confirm_remove(request, stdin_is_terminal, &mut input, &mut output)?;
    let result = execute::execute(&app, request)?;
    Ok(render::render(result))
}

fn maybe_confirm_remove(
    request: execute::CommandRequest,
    stdin_is_terminal: bool,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<execute::CommandRequest, TaskspaceError> {
    match request {
        execute::CommandRequest::Remove { name, yes, dry_run } => {
            if yes || dry_run || !stdin_is_terminal {
                return Ok(execute::CommandRequest::Remove { name, yes, dry_run });
            }

            if ask_remove_confirmation(name.as_str(), input, output)? {
                Ok(execute::CommandRequest::Remove {
                    name,
                    yes: true,
                    dry_run: false,
                })
            } else {
                Err(TaskspaceError::Usage("remove aborted by user".to_string()))
            }
        }
        _ => Ok(request),
    }
}

fn ask_remove_confirmation(
    name: &str,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> Result<bool, TaskspaceError> {
    write!(output, "remove session '{}'? [y/N]: ", name)
        .map_err(|err| TaskspaceError::Io(err.to_string()))?;
    output
        .flush()
        .map_err(|err| TaskspaceError::Io(err.to_string()))?;

    let mut answer_raw = String::new();
    input
        .read_line(&mut answer_raw)
        .map_err(|err| TaskspaceError::Io(err.to_string()))?;
    let answer = answer_raw.trim().to_ascii_lowercase();
    Ok(answer == "y" || answer == "yes")
}

fn render_completion(shell: Shell) -> Result<String, TaskspaceError> {
    let mut command = Cli::command();
    let mut output = Vec::new();
    generate(shell, &mut command, "taskspace", &mut output);
    String::from_utf8(output).map_err(|err| TaskspaceError::Internal(err.to_string()))
}

fn detect_shell() -> Shell {
    let shell_path = std::env::var_os("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    detect_shell_by_name(shell_name.as_str())
}

fn detect_shell_by_name(shell_name: &str) -> Shell {
    match shell_name {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "pwsh" | "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => Shell::Bash,
    }
}

enum ParseOutcome {
    Display(String),
    Usage(String),
}

fn parse_cli<I, T>(args: I) -> Result<Cli, ParseOutcome>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    match Cli::try_parse_from(args) {
        Ok(cli) => Ok(cli),
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                Err(ParseOutcome::Display(err.to_string()))
            }
            _ => Err(ParseOutcome::Usage(err.to_string())),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn run_new_list_archive_rm_doctor_flow() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();

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
    fn parse_help_and_version_as_display() {
        assert!(matches!(
            parse_cli(["taskspace", "-h"]),
            Err(ParseOutcome::Display(_))
        ));
        assert!(matches!(
            parse_cli(["taskspace", "-v"]),
            Err(ParseOutcome::Display(_))
        ));
    }

    #[test]
    fn parse_open_conflicting_arguments_reports_usage() {
        let err = run_with_args(["taskspace", "open", "demo", "--last"])
            .expect_err("name and --last should conflict");
        assert!(matches!(err, TaskspaceError::Usage(_)));
    }

    #[test]
    fn completion_outputs_shell_script() {
        let out =
            run_with_args(["taskspace", "completion", "bash"]).expect("completion should succeed");
        let script = out.join("\n");
        assert!(script.contains("taskspace"));
        assert!(script.contains("new"));
    }

    #[test]
    fn detect_shell_defaults_to_bash_for_unknown_shell() {
        assert!(matches!(detect_shell_by_name("unknown-shell"), Shell::Bash));
    }

    #[test]
    fn new_with_default_editors() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();

        // Test all default editor variants can be specified
        for editor in ["opencode", "codex", "claude"] {
            let name = format!("test-{}", editor);
            let out = run_with_args([
                "taskspace",
                "--root",
                root.to_str().expect("utf8"),
                "new",
                &name,
                "--editor",
                editor,
            ])
            .unwrap_or_else(|_| panic!("new with editor {} should succeed", editor));
            assert!(
                out[0].contains("created session"),
                "editor {} should create session",
                editor
            );
        }
    }

    #[test]
    fn new_with_template_path_succeeds() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();
        let repo = create_git_repo(&root, "seed-repo");
        let template_path = root.join("template.yaml");
        fs::write(
            &template_path,
            format!(
                "version: 1\nmanifest:\n  projects:\n    - id: app\n      source: {}\n      target: repos/app\n",
                repo.display()
            ),
        )
        .expect("write template");
        let template_path_text = path_to_string(&template_path);

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "new",
            "template-demo",
            "--template",
            template_path_text.as_str(),
        ])
        .expect("new with template should succeed");
        assert!(out[0].contains("created session"));
        assert!(root.join("template-demo/repos/app/.git").exists());
    }

    fn path_to_string(path: &Path) -> String {
        path.display().to_string()
    }

    fn create_git_repo(base: &Path, name: &str) -> std::path::PathBuf {
        let repo = base.join(name);
        fs::create_dir_all(&repo).expect("create repo dir");
        run_git(&repo, &["init", "-b", "main"]);
        fs::write(repo.join("README.md"), "seed repo\n").expect("write readme");
        run_git(&repo, &["add", "README.md"]);
        run_git(
            &repo,
            &[
                "-c",
                "user.name=taskspace",
                "-c",
                "user.email=taskspace@example.com",
                "commit",
                "-m",
                "initial",
            ],
        );
        repo
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {:?}", args);
    }

    #[test]
    fn open_with_unknown_editor_fails_gracefully() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();

        // Create a session first
        run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "new",
            "test-session",
        ])
        .expect("create test session");

        // Test with a definitely non-existent editor
        let err = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "open",
            "test-session",
            "--editor",
            "definitely-not-installed-editor-xyz",
        ])
        .expect_err("open with unknown editor should fail");

        // Should fail with usage error (unknown editor)
        assert!(
            format!("{err}").contains("unknown editor"),
            "should give unknown editor error, got: {}",
            err
        );
    }

    #[test]
    fn doctor_outputs_fail_label() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();

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

    #[test]
    fn maybe_confirm_remove_accepts_yes_in_interactive_mode() {
        let request = execute::CommandRequest::Remove {
            name: taskspace_core::SessionName::parse("demo").expect("name"),
            yes: false,
            dry_run: false,
        };
        let mut input = Cursor::new("y\n");
        let mut output = Vec::new();

        let confirmed = maybe_confirm_remove(request, true, &mut input, &mut output)
            .expect("confirmation should succeed");

        assert!(
            String::from_utf8(output)
                .expect("utf8")
                .contains("remove session 'demo'? [y/N]: ")
        );
        match confirmed {
            execute::CommandRequest::Remove { yes, .. } => assert!(yes),
            _ => panic!("expected remove command"),
        }
    }

    #[test]
    fn maybe_confirm_remove_declines_in_interactive_mode() {
        let request = execute::CommandRequest::Remove {
            name: taskspace_core::SessionName::parse("demo").expect("name"),
            yes: false,
            dry_run: false,
        };
        let mut input = Cursor::new("n\n");
        let mut output = Vec::new();

        let err = maybe_confirm_remove(request, true, &mut input, &mut output)
            .expect_err("remove should be aborted");
        assert!(matches!(err, TaskspaceError::Usage(_)));
        assert!(format!("{err}").contains("remove aborted by user"));
    }

    #[test]
    fn maybe_confirm_remove_keeps_yes_false_in_non_interactive_mode() {
        let request = execute::CommandRequest::Remove {
            name: taskspace_core::SessionName::parse("demo").expect("name"),
            yes: false,
            dry_run: false,
        };
        let mut input = Cursor::new("y\n");
        let mut output = Vec::new();

        let passthrough = maybe_confirm_remove(request, false, &mut input, &mut output)
            .expect("non-interactive should not prompt");

        assert!(output.is_empty());
        match passthrough {
            execute::CommandRequest::Remove { yes, .. } => assert!(!yes),
            _ => panic!("expected remove command"),
        }
    }
}
