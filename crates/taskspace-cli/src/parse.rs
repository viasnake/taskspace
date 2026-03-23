use taskspace_core::{RootAccess, RootIsolation, RootType, TaskState, TaskspaceError};

use crate::execute::CommandRequest;
use crate::{Commands, IsolationArg, RootTypeArg, TaskStateArg};

pub fn parse_command(command: Commands) -> Result<CommandRequest, TaskspaceError> {
    match command {
        Commands::Start { title, adapter } => Ok(CommandRequest::Start { title, adapter }),
        Commands::Attach {
            task,
            path,
            root_type,
            role,
            ro,
            rw,
            isolation,
        } => {
            if ro && rw {
                return Err(TaskspaceError::Usage(
                    "cannot set both --ro and --rw".to_string(),
                ));
            }
            let access = if ro { RootAccess::Ro } else { RootAccess::Rw };
            let root_type = map_root_type(root_type);
            let isolation = match isolation.unwrap_or(IsolationArg::Direct) {
                IsolationArg::Direct => RootIsolation::Direct,
                IsolationArg::Worktree => RootIsolation::Worktree,
                IsolationArg::Copy => RootIsolation::Copy,
                IsolationArg::Symlink => RootIsolation::Symlink,
                IsolationArg::Generated => RootIsolation::Generated,
            };
            Ok(CommandRequest::Attach {
                task_ref: task,
                path,
                root_type,
                role,
                access,
                isolation,
            })
        }
        Commands::Detach { task, root_id } => Ok(CommandRequest::Detach {
            task_ref: task,
            root_id,
        }),
        Commands::Enter { task, adapter } => Ok(CommandRequest::Enter {
            task_ref: task,
            adapter,
        }),
        Commands::List => Ok(CommandRequest::List),
        Commands::Show { task } => Ok(CommandRequest::Show { task_ref: task }),
        Commands::Verify { task } => Ok(CommandRequest::Verify { task_ref: task }),
        Commands::Finish { task, state } => {
            let state = match state.unwrap_or(TaskStateArg::Done) {
                TaskStateArg::Active => TaskState::Active,
                TaskStateArg::Blocked => TaskState::Blocked,
                TaskStateArg::Review => TaskState::Review,
                TaskStateArg::Done => TaskState::Done,
                TaskStateArg::Archived => TaskState::Archived,
            };
            Ok(CommandRequest::Finish {
                task_ref: task,
                state,
            })
        }
        Commands::Archive { task } => Ok(CommandRequest::Archive { task_ref: task }),
        Commands::Gc => Ok(CommandRequest::Gc),
        Commands::Completion { .. } | Commands::CompleteTasks => Err(TaskspaceError::Internal(
            "completion command should be handled before parse".to_string(),
        )),
    }
}

fn map_root_type(arg: RootTypeArg) -> RootType {
    match arg {
        RootTypeArg::Git => RootType::Git,
        RootTypeArg::Dir => RootType::Dir,
        RootTypeArg::File => RootType::File,
        RootTypeArg::Artifact => RootType::Artifact,
        RootTypeArg::Scratch => RootType::Scratch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Commands, IsolationArg, TaskStateArg};
    use std::path::PathBuf;

    #[test]
    fn attach_rejects_both_ro_and_rw() {
        let err = parse_command(Commands::Attach {
            task: "current".to_string(),
            path: PathBuf::from("/tmp"),
            root_type: RootTypeArg::Dir,
            role: "source".to_string(),
            ro: true,
            rw: true,
            isolation: None,
        })
        .expect_err("should fail");
        assert!(matches!(err, TaskspaceError::Usage(_)));
    }

    #[test]
    fn finish_maps_state_enum() {
        let parsed = parse_command(Commands::Finish {
            task: "current".to_string(),
            state: Some(TaskStateArg::Blocked),
        })
        .expect("parse");
        match parsed {
            CommandRequest::Finish { state, .. } => assert_eq!(state, TaskState::Blocked),
            _ => panic!("expected finish"),
        }
    }

    #[test]
    fn attach_maps_isolation_enum() {
        let parsed = parse_command(Commands::Attach {
            task: "current".to_string(),
            path: PathBuf::from("/tmp"),
            root_type: RootTypeArg::Git,
            role: "source".to_string(),
            ro: false,
            rw: false,
            isolation: Some(IsolationArg::Worktree),
        })
        .expect("parse");
        match parsed {
            CommandRequest::Attach { isolation, .. } => {
                assert_eq!(isolation, RootIsolation::Worktree)
            }
            _ => panic!("expected attach"),
        }
    }
}
