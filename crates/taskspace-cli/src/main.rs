use std::io::{BufRead, IsTerminal, Write};
use std::path::Path;
use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
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
        #[arg(long = "editor", value_delimiter = ',', num_args = 1..)]
        editors: Vec<String>,
    },
    Open {
        name: Option<String>,
        #[arg(long = "editor", value_delimiter = ',', num_args = 1..)]
        editors: Vec<String>,
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
        shell: Option<SupportedShell>,
    },
    #[command(name = "__complete-sessions", hide = true)]
    CompleteSessions,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum SupportedShell {
    Bash,
    Zsh,
    Fish,
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
        execute::CommandRequest::CompleteSessions => {
            let app = TaskspaceApp::new(root).map_err(execute::map_anyhow_error)?;
            return app.list_sessions().map_err(execute::map_anyhow_error);
        }
        other => other,
    };
    let app = TaskspaceApp::new(root).map_err(execute::map_anyhow_error)?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let stdin_is_terminal = stdin.is_terminal();
    let stdout_is_terminal = stdout.is_terminal();
    let mut input = stdin.lock();
    let mut output = std::io::stderr().lock();
    let request = maybe_confirm_remove(request, stdin_is_terminal, &mut input, &mut output)?;
    let guarded = maybe_guard_open(request, stdin_is_terminal, stdout_is_terminal)?;
    let result = execute::execute(&app, guarded.request)?;
    let mut lines = render::render(result);
    lines.extend(guarded.messages);
    Ok(lines)
}

#[derive(Debug)]
struct GuardedRequest {
    request: execute::CommandRequest,
    messages: Vec<String>,
}

fn maybe_guard_open(
    request: execute::CommandRequest,
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
) -> Result<GuardedRequest, TaskspaceError> {
    let context = OpenContext::from_env(stdin_is_terminal, stdout_is_terminal);
    maybe_guard_open_with_context(request, context)
}

fn maybe_guard_open_with_context(
    request: execute::CommandRequest,
    context: OpenContext,
) -> Result<GuardedRequest, TaskspaceError> {
    match request {
        execute::CommandRequest::New {
            name,
            template_path,
            open_after_create,
            editors,
            editors_explicit,
        } => {
            let normalized_editors = normalized_editor_list(editors);
            if !open_after_create {
                return Ok(GuardedRequest {
                    request: execute::CommandRequest::New {
                        name,
                        template_path,
                        open_after_create,
                        editors: normalized_editors,
                        editors_explicit,
                    },
                    messages: Vec::new(),
                });
            }

            match context.block_reason() {
                Some(reason) => Ok(GuardedRequest {
                    request: execute::CommandRequest::New {
                        name,
                        template_path,
                        open_after_create: false,
                        editors: normalized_editors,
                        editors_explicit,
                    },
                    messages: vec![format!("skipped opening session: {reason}")],
                }),
                None => Ok(GuardedRequest {
                    request: execute::CommandRequest::New {
                        name,
                        template_path,
                        open_after_create,
                        editors: normalized_editors,
                        editors_explicit,
                    },
                    messages: Vec::new(),
                }),
            }
        }
        execute::CommandRequest::Open {
            name,
            editors,
            editors_explicit,
        } => {
            let normalized_editors = normalized_editor_list(editors);
            match context.block_reason() {
                Some(reason) => Err(TaskspaceError::Usage(format!(
                    "cannot open session in this environment: {reason}"
                ))),
                None => Ok(GuardedRequest {
                    request: execute::CommandRequest::Open {
                        name,
                        editors: normalized_editors,
                        editors_explicit,
                    },
                    messages: Vec::new(),
                }),
            }
        }
        other => Ok(GuardedRequest {
            request: other,
            messages: Vec::new(),
        }),
    }
}

#[derive(Debug, Clone, Copy)]
struct OpenContext {
    stdin_is_terminal: bool,
    stdout_is_terminal: bool,
    has_ssh: bool,
    has_ci: bool,
}

impl OpenContext {
    fn from_env(stdin_is_terminal: bool, stdout_is_terminal: bool) -> Self {
        Self {
            stdin_is_terminal,
            stdout_is_terminal,
            has_ssh: ["SSH_CONNECTION", "SSH_CLIENT", "SSH_TTY"]
                .iter()
                .any(|key| std::env::var_os(key).is_some()),
            has_ci: std::env::var_os("CI").is_some(),
        }
    }

    fn block_reason(self) -> Option<&'static str> {
        if !self.stdin_is_terminal || !self.stdout_is_terminal {
            return Some("requires interactive terminal (TTY)");
        }
        if self.has_ssh {
            return Some("detected SSH session");
        }
        if self.has_ci {
            return Some("detected CI environment");
        }
        None
    }
}

fn normalized_editor_list(editors: Vec<String>) -> Vec<String> {
    editors
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
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

fn render_completion(shell: SupportedShell) -> Result<String, TaskspaceError> {
    let script = match shell {
        SupportedShell::Bash => BASH_COMPLETION,
        SupportedShell::Zsh => ZSH_COMPLETION,
        SupportedShell::Fish => FISH_COMPLETION,
    };
    Ok(script.to_string())
}

fn detect_shell() -> SupportedShell {
    let shell_path = std::env::var_os("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    detect_shell_by_name(shell_name.as_str())
}

fn detect_shell_by_name(shell_name: &str) -> SupportedShell {
    match shell_name {
        "bash" => SupportedShell::Bash,
        "zsh" => SupportedShell::Zsh,
        "fish" => SupportedShell::Fish,
        _ => SupportedShell::Bash,
    }
}

const BASH_COMPLETION: &str = r#"_taskspace() {
    local cur cmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    cmd="${COMP_WORDS[1]}"

    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "new open list ls rm remove archive doctor completion" -- "$cur") )
        return 0
    fi

    case "$cmd" in
        open|rm|remove|archive)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-sessions 2>/dev/null)" -- "$cur") )
            fi
            ;;
        completion)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "bash zsh fish" -- "$cur") )
            fi
            ;;
    esac
}

complete -F _taskspace taskspace
"#;

const ZSH_COMPLETION: &str = r#"#compdef taskspace

_taskspace() {
    local -a commands sessions shells
    commands=(new open list ls rm remove archive doctor completion)
    shells=(bash zsh fish)

    if (( CURRENT == 2 )); then
        compadd -a commands
        return
    fi

    case "$words[2]" in
        open|rm|remove|archive)
            if (( CURRENT == 3 )); then
                sessions=("${(@f)$(taskspace __complete-sessions 2>/dev/null)}")
                compadd -a sessions
            fi
            ;;
        completion)
            if (( CURRENT == 3 )); then
                compadd -a shells
            fi
            ;;
    esac
}

compdef _taskspace taskspace
"#;

const FISH_COMPLETION: &str = r#"complete -c taskspace -f
complete -c taskspace -n "not __fish_seen_subcommand_from new open list ls rm remove archive doctor completion" -a "new open list ls rm remove archive doctor completion"
complete -c taskspace -n "__fish_seen_subcommand_from open rm remove archive" -a "(taskspace __complete-sessions 2>/dev/null)"
complete -c taskspace -n "__fish_seen_subcommand_from completion" -a "bash zsh fish"
"#;

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
        assert!(script.contains("__complete-sessions"));
    }

    #[test]
    fn detect_shell_defaults_to_bash_for_unknown_shell() {
        assert!(matches!(
            detect_shell_by_name("unknown-shell"),
            SupportedShell::Bash
        ));
    }

    #[test]
    fn completion_rejects_unsupported_shell() {
        let err = run_with_args(["taskspace", "completion", "powershell"])
            .expect_err("powershell should be unsupported");
        assert!(matches!(err, TaskspaceError::Usage(_)));
    }

    #[test]
    fn complete_sessions_outputs_only_session_names() {
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

        let out = run_with_args([
            "taskspace",
            "--root",
            root.to_str().expect("utf8"),
            "__complete-sessions",
        ])
        .expect("complete sessions should succeed");

        assert_eq!(out, vec!["demo".to_string()]);
    }

    #[test]
    fn new_with_default_editors() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().to_path_buf();

        // Test all default editor variants can be specified
        for (editor, _) in taskspace_core::default_editors() {
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
    fn maybe_guard_open_blocks_open_in_non_interactive_context() {
        let request = execute::CommandRequest::Open {
            name: Some(taskspace_core::SessionName::parse("demo").expect("name")),
            editors: vec!["vscode".to_string()],
            editors_explicit: true,
        };
        let context = OpenContext {
            stdin_is_terminal: false,
            stdout_is_terminal: true,
            has_ssh: false,
            has_ci: false,
        };

        let err = maybe_guard_open_with_context(request, context)
            .expect_err("open should fail without interactive TTY");
        assert!(format!("{err}").contains("interactive terminal"));
    }

    #[test]
    fn maybe_guard_open_skips_new_open_in_ssh_context() {
        let request = execute::CommandRequest::New {
            name: taskspace_core::SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: true,
            editors: vec!["vscode".to_string()],
            editors_explicit: true,
        };
        let context = OpenContext {
            stdin_is_terminal: true,
            stdout_is_terminal: true,
            has_ssh: true,
            has_ci: false,
        };

        let guarded = maybe_guard_open_with_context(request, context)
            .expect("new --open should be downgraded to create-only");
        assert_eq!(guarded.messages.len(), 1);
        assert!(guarded.messages[0].contains("detected SSH session"));

        match guarded.request {
            execute::CommandRequest::New {
                open_after_create,
                editors,
                editors_explicit,
                ..
            } => {
                assert!(!open_after_create);
                assert_eq!(editors, vec!["vscode".to_string()]);
                assert!(editors_explicit);
            }
            _ => panic!("expected new command"),
        }
    }

    #[test]
    fn maybe_guard_open_keeps_implicit_editor_flag_for_open() {
        let request = execute::CommandRequest::Open {
            name: Some(taskspace_core::SessionName::parse("demo").expect("name")),
            editors: Vec::new(),
            editors_explicit: false,
        };
        let context = OpenContext {
            stdin_is_terminal: true,
            stdout_is_terminal: true,
            has_ssh: false,
            has_ci: false,
        };

        let guarded = maybe_guard_open_with_context(request, context)
            .expect("open should proceed in interactive local context");

        match guarded.request {
            execute::CommandRequest::Open {
                editors,
                editors_explicit,
                ..
            } => {
                assert!(editors.is_empty());
                assert!(!editors_explicit);
            }
            _ => panic!("expected open command"),
        }
    }

    #[test]
    fn normalized_editor_list_keeps_empty_when_not_specified() {
        let normalized = normalized_editor_list(Vec::new());
        assert!(normalized.is_empty());
    }

    #[test]
    fn normalized_editor_list_trims_and_removes_empty_values() {
        let normalized = normalized_editor_list(vec![
            "  vscode ".to_string(),
            "".to_string(),
            " opencode".to_string(),
            "   ".to_string(),
        ]);

        assert_eq!(
            normalized,
            vec!["vscode".to_string(), "opencode".to_string()]
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

        fs::remove_file(root.join("demo/AGENTS.md")).expect("remove file");
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
