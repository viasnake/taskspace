use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use taskspace_app::TaskspaceApp;
use taskspace_core::TaskspaceError;

mod exit_code;
mod render;

#[derive(Parser)]
#[command(name = "taskspace")]
#[command(version, about = "Reusable git checkout slots for AI agent work")]
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
    Init {
        source: PathBuf,
        #[arg(long)]
        slots: Option<u16>,
    },
    List,
    Show {
        slot: String,
    },
    Checkout {
        slot: String,
        git_ref: String,
    },
    Enter {
        slot: String,
        #[arg(long)]
        agent: Option<String>,
    },
    HookContext {
        path: Option<PathBuf>,
    },
    Completion {
        #[arg(value_enum)]
        shell: Option<SupportedShell>,
    },
    #[command(name = "__complete-slots", hide = true)]
    CompleteSlots,
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
    let app = TaskspaceApp::new(cli.root).map_err(map_anyhow_error)?;
    match cli.command {
        Commands::Completion { shell } => {
            let selected = shell.unwrap_or_else(detect_shell);
            Ok(vec![render_completion(selected)])
        }
        Commands::CompleteSlots => {
            let slots = app.list_slots().map_err(map_anyhow_error)?;
            Ok(slots
                .into_iter()
                .map(|slot| slot.id.as_str().to_string())
                .collect())
        }
        Commands::Init { source, slots } => app
            .init_workspaces(&source.display().to_string(), slots)
            .map(render::initialized)
            .map_err(map_anyhow_error),
        Commands::List => app
            .list_slots()
            .map(render::slot_list)
            .map_err(map_anyhow_error),
        Commands::Show { slot } => app
            .show_slot(&slot)
            .map(render::slot_detail)
            .map_err(map_anyhow_error),
        Commands::Checkout { slot, git_ref } => app
            .checkout(&slot, &git_ref)
            .map(render::checked_out)
            .map_err(map_anyhow_error),
        Commands::Enter { slot, agent } => app
            .enter_slot(&slot, agent.as_deref())
            .map(|result| render::entered(&result.agent, &result.cwd, &result.slot_id))
            .map_err(map_anyhow_error),
        Commands::HookContext { path } => app
            .hook_context(path)
            .map(render::hook_context)
            .map_err(map_anyhow_error),
    }
}

fn map_anyhow_error(err: anyhow::Error) -> TaskspaceError {
    match err.downcast::<TaskspaceError>() {
        Ok(ts_err) => ts_err,
        Err(other) => TaskspaceError::Internal(other.to_string()),
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
        COMPREPLY=( $(compgen -W "init list show checkout enter hook-context completion" -- "$cur") )
        return 0
    fi

    case "$cmd" in
        show|checkout|enter)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-slots 2>/dev/null)" -- "$cur") )
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
    local -a commands slots shells
    commands=(init list show checkout enter hook-context completion)
    shells=(bash zsh fish)

    if (( CURRENT == 2 )); then
        compadd -a commands
        return
    fi

    case "$words[2]" in
        show|checkout|enter)
            if (( CURRENT == 3 )); then
                slots=("${(@f)$(taskspace __complete-slots 2>/dev/null)}")
                compadd -a slots
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
complete -c taskspace -n "not __fish_seen_subcommand_from init list show checkout enter hook-context completion" -a "init list show checkout enter hook-context completion"
complete -c taskspace -n "__fish_seen_subcommand_from show checkout enter" -a "(taskspace __complete-slots 2>/dev/null)"
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
        assert_eq!(out, vec!["no slots found".to_string()]);
    }
}
