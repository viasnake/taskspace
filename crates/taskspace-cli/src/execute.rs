use std::path::PathBuf;

use taskspace_app::{
    ArchiveSessionRequest, DoctorReport, NewSessionRequest, OpenSessionRequest, OpenSessionTarget,
    RemoveSessionRequest, TaskspaceApp,
};
use taskspace_core::{SessionName, TaskspaceError};

use crate::SupportedShell;

#[derive(Debug, Clone)]
pub enum CommandRequest {
    New {
        name: SessionName,
        template_path: Option<PathBuf>,
        open_after_create: bool,
        editor: String,
    },
    Open {
        name: Option<SessionName>,
        editor: String,
    },
    List,
    Remove {
        name: SessionName,
        yes: bool,
        dry_run: bool,
    },
    Archive {
        name: SessionName,
    },
    Doctor,
    Completion {
        shell: Option<SupportedShell>,
    },
    CompleteSessions,
}

pub enum CommandResult {
    None,
    Created(PathBuf),
    SessionList(Vec<String>),
    Removed { name: String, dry_run: bool },
    Archived(PathBuf),
    Doctor(DoctorReport),
}

pub fn execute(
    app: &TaskspaceApp,
    command: CommandRequest,
) -> Result<CommandResult, TaskspaceError> {
    match command {
        CommandRequest::New {
            name,
            template_path,
            open_after_create,
            editor,
        } => {
            let created = app
                .create_session(NewSessionRequest {
                    name,
                    template_path,
                    open_after_create,
                    editor,
                })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Created(created))
        }
        CommandRequest::Open { name, editor } => {
            let target = match name {
                Some(name) => OpenSessionTarget::Name(name),
                None => OpenSessionTarget::Last,
            };
            app.open_session(OpenSessionRequest { target, editor })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::None)
        }
        CommandRequest::List => {
            let sessions = app.list_sessions().map_err(map_anyhow_error)?;
            Ok(CommandResult::SessionList(sessions))
        }
        CommandRequest::Doctor => {
            let report = app.doctor().map_err(map_anyhow_error)?;
            Ok(CommandResult::Doctor(report))
        }
        CommandRequest::Remove { name, yes, dry_run } => {
            let name_text = name.as_str().to_string();
            app.remove_session(RemoveSessionRequest { name, yes, dry_run })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Removed {
                name: name_text,
                dry_run,
            })
        }
        CommandRequest::Archive { name } => {
            let archived = app
                .archive_session(ArchiveSessionRequest { name })
                .map_err(map_anyhow_error)?;
            Ok(CommandResult::Archived(archived))
        }
        CommandRequest::Completion { .. } => Err(TaskspaceError::Internal(
            "completion command should be handled before command execution".to_string(),
        )),
        CommandRequest::CompleteSessions => Err(TaskspaceError::Internal(
            "complete-sessions command should be handled before command execution".to_string(),
        )),
    }
}

pub fn map_anyhow_error(err: anyhow::Error) -> TaskspaceError {
    match err.downcast::<TaskspaceError>() {
        Ok(ts_err) => ts_err,
        Err(other) => TaskspaceError::Internal(other.to_string()),
    }
}
