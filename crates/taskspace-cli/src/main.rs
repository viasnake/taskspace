use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use taskspace_app::TaskspaceApp;
use taskspace_core::TaskspaceError;

mod execute;
mod exit_code;
mod parse;
mod render;

#[derive(Parser)]
#[command(name = "taskspace")]
#[command(version, about = "Task-oriented multi-root workspace manager")]
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
    Start {
        title: String,
        #[arg(long)]
        adapter: Option<String>,
    },
    Attach {
        task: String,
        path: PathBuf,
        #[arg(long = "type", value_enum)]
        root_type: RootTypeArg,
        #[arg(long)]
        role: String,
        #[arg(long, action = ArgAction::SetTrue)]
        ro: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        rw: bool,
        #[arg(long, value_enum)]
        isolation: Option<IsolationArg>,
    },
    Detach {
        task: String,
        root_id: String,
    },
    Enter {
        task: String,
        #[arg(long)]
        adapter: Option<String>,
    },
    List,
    Show {
        task: String,
    },
    Verify {
        task: String,
    },
    Finish {
        task: String,
        #[arg(long, value_enum)]
        state: Option<TaskStateArg>,
    },
    Archive {
        task: String,
    },
    Gc,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum RootTypeArg {
    Git,
    Dir,
    File,
    Artifact,
    Scratch,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum IsolationArg {
    Direct,
    Worktree,
    Copy,
    Symlink,
    Generated,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum TaskStateArg {
    Active,
    Blocked,
    Review,
    Done,
    Archived,
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
    let request = parse::parse_command(cli.command)?;
    let app = TaskspaceApp::new(cli.root).map_err(execute::map_anyhow_error)?;
    let result = execute::execute(&app, request)?;
    Ok(render::render(result))
}

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
    fn start_and_show_work() {
        let temp = tempdir().expect("temp");
        let root = Some(temp.path().to_path_buf());
        let started = run_with_cli(Cli {
            _version: None,
            root: root.clone(),
            command: Commands::Start {
                title: "demo".to_string(),
                adapter: None,
            },
        })
        .expect("start");
        assert!(started[0].contains("started task:"));

        let list = run_with_cli(Cli {
            _version: None,
            root,
            command: Commands::List,
        })
        .expect("list");
        assert_eq!(list.len(), 1);
    }
}
