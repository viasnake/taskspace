use std::path::PathBuf;

use taskspace_app::{
    EnterTaskRequest, FinishTaskRequest, GcResult, ShowTaskRequest, StartTaskRequest, TaskSummary,
    TaskspaceApp, UseReposRequest,
};
use taskspace_core::{Task, TaskState, TaskspaceError};

#[derive(Debug, Clone)]
pub enum CommandRequest {
    New {
        title: String,
    },
    Repos,
    Use {
        task_ref: String,
        repos: Vec<String>,
    },
    Enter {
        task_ref: String,
    },
    List,
    Show {
        task_ref: String,
    },
    Finish {
        task_ref: String,
        state: TaskState,
    },
    Gc,
}

pub enum CommandResult {
    Started(Task),
    Repos(Vec<String>),
    Scoped(Task),
    Entered {
        adapter: String,
        cwd: PathBuf,
        task_id: String,
    },
    TaskList(Vec<TaskSummary>),
    TaskDetail(Task),
    Finished(TaskState),
    Gc(GcResult),
}

pub fn execute(
    app: &TaskspaceApp,
    command: CommandRequest,
) -> Result<CommandResult, TaskspaceError> {
    match command {
        CommandRequest::New { title } => {
            let task = app
                .start_task(StartTaskRequest {
                    title,
                    entry_adapter: None,
                })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Started(task))
        }
        CommandRequest::Repos => {
            let repos = app.list_repos().map_err(map_anyhow_error)?;
            Ok(CommandResult::Repos(repos))
        }
        CommandRequest::Use { task_ref, repos } => {
            let task = app
                .use_repos(UseReposRequest { task_ref, repos })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Scoped(task))
        }
        CommandRequest::Enter { task_ref } => {
            let result = app
                .enter_task(EnterTaskRequest { task_ref })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Entered {
                adapter: result.adapter,
                cwd: result.cwd,
                task_id: result.task_id,
            })
        }
        CommandRequest::List => {
            let tasks = app.list_tasks().map_err(map_anyhow_error)?;
            Ok(CommandResult::TaskList(tasks))
        }
        CommandRequest::Show { task_ref } => {
            let task = app
                .show_task(ShowTaskRequest { task_ref })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::TaskDetail(task))
        }
        CommandRequest::Finish { task_ref, state } => {
            let state = app
                .finish_task(FinishTaskRequest {
                    task_ref,
                    target_state: state,
                })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Finished(state))
        }
        CommandRequest::Gc => {
            let result = app.gc().map_err(map_anyhow_error)?;
            Ok(CommandResult::Gc(result))
        }
    }
}

pub fn map_anyhow_error(err: anyhow::Error) -> TaskspaceError {
    match err.downcast::<TaskspaceError>() {
        Ok(ts_err) => ts_err,
        Err(other) => TaskspaceError::Internal(other.to_string()),
    }
}
