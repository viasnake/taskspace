use std::path::PathBuf;

use taskspace_app::{
    ArchiveTaskRequest, AttachRootRequest, DetachRootRequest, EnterTaskRequest, FinishTaskRequest,
    GcResult, ShowTaskRequest, StartTaskRequest, TaskSummary, TaskspaceApp, VerifyTaskRequest,
};
use taskspace_core::{RootAccess, RootIsolation, RootType, Task, TaskState, TaskspaceError};

#[derive(Debug, Clone)]
pub enum CommandRequest {
    Start {
        title: String,
        adapter: Option<String>,
    },
    Attach {
        task_ref: String,
        path: PathBuf,
        root_type: RootType,
        role: String,
        access: RootAccess,
        isolation: RootIsolation,
    },
    Detach {
        task_ref: String,
        root_id: String,
    },
    Enter {
        task_ref: String,
        adapter: Option<String>,
    },
    List,
    Show {
        task_ref: String,
    },
    Verify {
        task_ref: String,
    },
    Finish {
        task_ref: String,
        state: TaskState,
    },
    Archive {
        task_ref: String,
    },
    Gc,
}

pub enum CommandResult {
    Started(Task),
    Attached {
        root_id: String,
        warnings: Vec<String>,
    },
    Detached,
    Entered {
        adapter: String,
        cwd: PathBuf,
        task_id: String,
    },
    TaskList(Vec<TaskSummary>),
    TaskDetail(Task),
    Verified {
        task_id: String,
        ran: Vec<String>,
    },
    Finished(TaskState),
    Archived,
    Gc(GcResult),
}

pub fn execute(
    app: &TaskspaceApp,
    command: CommandRequest,
) -> Result<CommandResult, TaskspaceError> {
    match command {
        CommandRequest::Start { title, adapter } => {
            let task = app
                .start_task(StartTaskRequest {
                    title,
                    entry_adapter: adapter,
                })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Started(task))
        }
        CommandRequest::Attach {
            task_ref,
            path,
            root_type,
            role,
            access,
            isolation,
        } => {
            let result = app
                .attach_root(AttachRootRequest {
                    task_ref,
                    root_type,
                    path,
                    role,
                    access,
                    isolation,
                })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Attached {
                root_id: result.root_id,
                warnings: result.warnings,
            })
        }
        CommandRequest::Detach { task_ref, root_id } => {
            app.detach_root(DetachRootRequest { task_ref, root_id })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Detached)
        }
        CommandRequest::Enter { task_ref, adapter } => {
            let result = app
                .enter_task(EnterTaskRequest { task_ref, adapter })
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
        CommandRequest::Verify { task_ref } => {
            let result = app
                .verify_task(VerifyTaskRequest { task_ref })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Verified {
                task_id: result.task_id,
                ran: result.ran,
            })
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
        CommandRequest::Archive { task_ref } => {
            app.archive_task(ArchiveTaskRequest { task_ref })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Archived)
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
