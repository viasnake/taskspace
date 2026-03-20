use std::path::PathBuf;

use taskspace_app::{
    ArchiveSessionRequest, DoctorReport, NewSessionRequest, OpenSessionRequest, OpenSessionTarget,
    RemoveSessionRequest, TaskspaceApp,
};
use taskspace_core::{EditorKind, RepoSpec, SessionName, TaskspaceError};

#[derive(Debug, Clone)]
pub enum CommandRequest {
    New {
        name: SessionName,
        repos: Vec<RepoSpec>,
        open_after_create: bool,
        editor: EditorKind,
    },
    Open {
        name: Option<SessionName>,
        editor: EditorKind,
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
            repos,
            open_after_create,
            editor,
        } => {
            let created = app
                .create_session(NewSessionRequest {
                    name,
                    repos,
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
    }
}

pub fn map_anyhow_error(err: anyhow::Error) -> TaskspaceError {
    match err.downcast::<TaskspaceError>() {
        Ok(ts_err) => ts_err,
        Err(other) => TaskspaceError::Internal(other.to_string()),
    }
}
