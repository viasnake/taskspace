use std::path::{Path, PathBuf};

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use taskspace_app::TaskspaceApp;
use taskspace_core::TaskspaceError;

mod exit_code;
mod render;

#[derive(Parser)]
#[command(name = "taskspace")]
#[command(version, about = "Dynamic git workspace slots for AI agent work")]
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
    Init,
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    Slot {
        #[command(subcommand)]
        command: SlotCommands,
    },
    Sync {
        project: Option<String>,
        #[arg(long)]
        all: bool,
    },
    Enter {
        slot: String,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long)]
        no_sync: bool,
    },
    HookContext {
        path: Option<PathBuf>,
    },
    Completion {
        #[arg(value_enum)]
        shell: Option<SupportedShell>,
    },
    #[command(name = "__complete-projects", hide = true)]
    CompleteProjects,
    #[command(name = "__complete-slot-refs", hide = true)]
    CompleteSlotRefs,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum ProjectCommands {
    Add { project: String, source: PathBuf },
    List,
    Show { project: String },
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum SlotCommands {
    Add {
        project: String,
        #[arg(long)]
        count: Option<u16>,
    },
    List {
        project: Option<String>,
    },
    Show {
        slot: String,
    },
    Remove {
        slot: String,
        #[arg(long)]
        force: bool,
    },
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
        Commands::Init => app
            .init_workspace()
            .map(render::initialized)
            .map_err(map_anyhow_error),
        Commands::Project { command } => run_project_command(&app, command),
        Commands::Slot { command } => run_slot_command(&app, command),
        Commands::Sync { project, all } => {
            if all {
                if project.is_some() {
                    return Err(TaskspaceError::Usage(
                        "use either <project> or --all".to_string(),
                    ));
                }
                render_sync_result(app.sync_all().map_err(map_anyhow_error)?)
            } else {
                let Some(project) = project else {
                    return Err(TaskspaceError::Usage(
                        "sync requires <project> or --all".to_string(),
                    ));
                };
                render_sync_result(app.sync_project(&project).map_err(map_anyhow_error)?)
            }
        }
        Commands::Enter {
            slot,
            agent,
            no_sync,
        } => app
            .enter_slot(&slot, agent.as_deref(), !no_sync)
            .map(|result| render::entered(&result))
            .map_err(map_anyhow_error),
        Commands::HookContext { path } => app
            .hook_context(path)
            .map(render::hook_context)
            .map_err(map_anyhow_error),
        Commands::Completion { shell } => {
            let selected = shell.unwrap_or_else(detect_shell);
            Ok(vec![render_completion(selected)])
        }
        Commands::CompleteProjects => app
            .list_projects()
            .map(|projects| {
                projects
                    .into_iter()
                    .map(|project| project.id.as_str().to_string())
                    .collect()
            })
            .map_err(map_anyhow_error),
        Commands::CompleteSlotRefs => app
            .list_slots()
            .map(|slots| {
                slots
                    .into_iter()
                    .map(|slot| slot.slot_ref().as_string())
                    .collect()
            })
            .map_err(map_anyhow_error),
    }
}

fn run_project_command(
    app: &TaskspaceApp,
    command: ProjectCommands,
) -> Result<Vec<String>, TaskspaceError> {
    match command {
        ProjectCommands::Add { project, source } => app
            .add_project(&project, &source.display().to_string())
            .map(render::project_added)
            .map_err(map_anyhow_error),
        ProjectCommands::List => app
            .list_projects()
            .map(render::project_list)
            .map_err(map_anyhow_error),
        ProjectCommands::Show { project } => app
            .show_project(&project)
            .map(render::project_detail)
            .map_err(map_anyhow_error),
    }
}

fn run_slot_command(
    app: &TaskspaceApp,
    command: SlotCommands,
) -> Result<Vec<String>, TaskspaceError> {
    match command {
        SlotCommands::Add { project, count } => app
            .add_slots(&project, count)
            .map(render::slots_added)
            .map_err(map_anyhow_error),
        SlotCommands::List { project } => match project {
            Some(project) => app
                .list_slots_for_project(&project)
                .map(render::slot_list)
                .map_err(map_anyhow_error),
            None => app
                .list_slots()
                .map(render::slot_list)
                .map_err(map_anyhow_error),
        },
        SlotCommands::Show { slot } => app
            .show_slot(&slot)
            .map(render::slot_detail)
            .map_err(map_anyhow_error),
        SlotCommands::Remove { slot, force } => app
            .remove_slot(&slot, force)
            .map(render::slot_removed)
            .map_err(map_anyhow_error),
    }
}

fn render_sync_result(
    result: taskspace_app::SyncSlotsResult,
) -> Result<Vec<String>, TaskspaceError> {
    if result.has_failures() {
        Err(TaskspaceError::ExternalCommand(render::sync_error(&result)))
    } else {
        Ok(render::sync_result(&result))
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
    local cur prev cmd subcmd
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    cmd="${COMP_WORDS[1]}"
    subcmd="${COMP_WORDS[2]}"

    if [[ ${COMP_CWORD} -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "init project slot sync enter hook-context completion" -- "$cur") )
        return 0
    fi

    case "$cmd" in
        project)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "add list show" -- "$cur") )
            elif [[ "$subcmd" == "show" && ${COMP_CWORD} -eq 3 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-projects 2>/dev/null)" -- "$cur") )
            fi
            ;;
        slot)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "add list show remove" -- "$cur") )
            elif [[ "$subcmd" == "add" && ${COMP_CWORD} -eq 3 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-projects 2>/dev/null)" -- "$cur") )
            elif [[ "$subcmd" == "list" && ${COMP_CWORD} -eq 3 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-projects 2>/dev/null)" -- "$cur") )
            elif [[ ( "$subcmd" == "show" || "$subcmd" == "remove" ) && ${COMP_CWORD} -eq 3 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-slot-refs 2>/dev/null)" -- "$cur") )
            fi
            ;;
        sync)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "--all $(taskspace __complete-projects 2>/dev/null)" -- "$cur") )
            fi
            ;;
        enter)
            if [[ ${COMP_CWORD} -eq 2 ]]; then
                COMPREPLY=( $(compgen -W "$(taskspace __complete-slot-refs 2>/dev/null)" -- "$cur") )
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
    local -a commands project_sub slot_sub projects slots shells
    commands=(init project slot sync enter hook-context completion)
    project_sub=(add list show)
    slot_sub=(add list show remove)
    shells=(bash zsh fish)

    if (( CURRENT == 2 )); then
        compadd -a commands
        return
    fi

    case "$words[2]" in
        project)
            if (( CURRENT == 3 )); then
                compadd -a project_sub
            elif [[ "$words[3]" == "show" && CURRENT == 4 ]]; then
                projects=("${(@f)$(taskspace __complete-projects 2>/dev/null)}")
                compadd -a projects
            fi
            ;;
        slot)
            if (( CURRENT == 3 )); then
                compadd -a slot_sub
            elif [[ ( "$words[3]" == "add" || "$words[3]" == "list" ) && CURRENT == 4 ]]; then
                projects=("${(@f)$(taskspace __complete-projects 2>/dev/null)}")
                compadd -a projects
            elif [[ ( "$words[3]" == "show" || "$words[3]" == "remove" ) && CURRENT == 4 ]]; then
                slots=("${(@f)$(taskspace __complete-slot-refs 2>/dev/null)}")
                compadd -a slots
            fi
            ;;
        sync)
            if (( CURRENT == 3 )); then
                projects=("${(@f)$(taskspace __complete-projects 2>/dev/null)}")
                compadd -- --all
                compadd -a projects
            fi
            ;;
        enter)
            if (( CURRENT == 3 )); then
                slots=("${(@f)$(taskspace __complete-slot-refs 2>/dev/null)}")
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
complete -c taskspace -n "not __fish_seen_subcommand_from init project slot sync enter hook-context completion" -a "init project slot sync enter hook-context completion"
complete -c taskspace -n "__fish_seen_subcommand_from project; and not __fish_seen_subcommand_from add list show" -a "add list show"
complete -c taskspace -n "__fish_seen_subcommand_from slot; and not __fish_seen_subcommand_from add list show remove" -a "add list show remove"
complete -c taskspace -n "__fish_seen_subcommand_from project show" -a "(taskspace __complete-projects 2>/dev/null)"
complete -c taskspace -n "__fish_seen_subcommand_from slot add slot list" -a "(taskspace __complete-projects 2>/dev/null)"
complete -c taskspace -n "__fish_seen_subcommand_from slot show slot remove enter" -a "(taskspace __complete-slot-refs 2>/dev/null)"
complete -c taskspace -n "__fish_seen_subcommand_from sync" -a "(taskspace __complete-projects 2>/dev/null)"
complete -c taskspace -n "__fish_seen_subcommand_from completion" -a "bash zsh fish"
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn project_list_empty_works() {
        let temp = tempdir().expect("temp");
        let out = run_with_cli(Cli {
            _version: None,
            root: Some(temp.path().to_path_buf()),
            command: Commands::Project {
                command: ProjectCommands::List,
            },
        })
        .expect("project list");
        assert_eq!(out, vec!["no projects found".to_string()]);
    }
}
