use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use taskspace_app::TaskspaceApp;
use taskspace_core::TaskspaceError;

mod execute;
mod exit_code;
mod parse;
mod render;

#[derive(Parser)]
#[command(name = "taskspace")]
#[command(version, about = "Minimal task launcher for AI work")]
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

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum Commands {
    New {
        title: String,
    },
    Repos,
    Use {
        task: String,
        repos: Vec<String>,
    },
    Enter {
        task: String,
    },
    List,
    Show {
        task: String,
    },
    Finish {
        task: String,
        #[arg(long, value_enum)]
        state: Option<TaskStateArg>,
    },
    Gc,
    Completion {
        #[arg(value_enum)]
        shell: Option<SupportedShell>,
    },
    #[command(name = "__complete-tasks", hide = true)]
    CompleteTasks,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum TaskStateArg {
    Blocked,
    Review,
    Done,
    Archived,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum SupportedShell {
    Bash,
    Zsh,
    Fish,
}

fn main() {
    let cli = Cli::parse();
    match run_with_cli(cli) {
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

fn run_with_cli(cli: Cli) -> Result<Vec<String>, TaskspaceError> {
    let app = TaskspaceApp::new(cli.root).map_err(execute::map_anyhow_error)?;
    match cli.command {
        Commands::Completion { shell } => {
            let selected = shell.unwrap_or_else(detect_shell);
            Ok(vec![render_completion(selected)])
        }
        Commands::CompleteTasks => {
            let tasks = app.list_tasks().map_err(execute::map_anyhow_error)?;
            Ok(tasks.into_iter().map(|task| task.id).collect())
        }
        command => {
            let request = parse::parse_command(command)?;
            let result = execute::execute(&app, request)?;
            Ok(render::render(result))
        }
    }
}

fn render_completion(shell: SupportedShell) -> String {
    match shell {
        SupportedShell::Bash => BASH_COMPLETION.to_string(),
        SupportedShell::Zsh => ZSH_COMPLETION.to_string(),
        SupportedShell::Fish => FISH_COMPLETION.to_string(),
    }
}

fn detect_shell() -> SupportedShell {
    let shell_path = std::env::var_os("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match shell_name.as_str() {
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
        COMPREPLY=( $(compgen -W "new repos use enter list show finish gc completion" -- "$cur") )
        return 0
    fi

    case "$cmd" in
        use|enter|show|finish)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-tasks 2>/dev/null) current" -- "$cur") )
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
    local -a commands tasks shells
    commands=(new repos use enter list show finish gc completion)
    shells=(bash zsh fish)

    if (( CURRENT == 2 )); then
        compadd -a commands
        return
    fi

    case "$words[2]" in
        use|enter|show|finish)
            if (( CURRENT == 3 )); then
                tasks=("${(@f)$(taskspace __complete-tasks 2>/dev/null)}" "current")
                compadd -a tasks
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
complete -c taskspace -n "not __fish_seen_subcommand_from new repos use enter list show finish gc completion" -a "new repos use enter list show finish gc completion"
complete -c taskspace -n "__fish_seen_subcommand_from use enter show finish" -a "(taskspace __complete-tasks 2>/dev/null) current"
complete -c taskspace -n "__fish_seen_subcommand_from completion" -a "bash zsh fish"
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn list_empty_works() {
        let temp = tempdir().expect("temp");
        let out = run_with_cli(Cli {
            _version: None,
            root: Some(temp.path().to_path_buf()),
            command: Commands::List,
        })
        .expect("list");
        assert_eq!(out, vec!["no tasks found".to_string()]);
    }

    #[test]
    fn new_and_list_work() {
        let temp = tempdir().expect("temp");
        let root = Some(temp.path().to_path_buf());
        let started = run_with_cli(Cli {
            _version: None,
            root: root.clone(),
            command: Commands::New {
                title: "demo".to_string(),
            },
        })
        .expect("new");
        assert!(started[0].starts_with("tsk_"));

        let list = run_with_cli(Cli {
            _version: None,
            root,
            command: Commands::List,
        })
        .expect("list");
        assert_eq!(list.len(), 1);
    }
}
