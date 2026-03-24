use taskspace_core::{TaskState, TaskspaceError};

use crate::execute::CommandRequest;
use crate::{Commands, TaskStateArg};

pub fn parse_command(command: Commands) -> Result<CommandRequest, TaskspaceError> {
    match command {
        Commands::New { title } => Ok(CommandRequest::New { title }),
        Commands::Repos => Ok(CommandRequest::Repos),
        Commands::Use { task, repos } => Ok(CommandRequest::Use {
            task_ref: task,
            repos,
        }),
        Commands::Enter { task } => Ok(CommandRequest::Enter { task_ref: task }),
        Commands::List => Ok(CommandRequest::List),
        Commands::Show { task } => Ok(CommandRequest::Show { task_ref: task }),
        Commands::Finish { task, state } => {
            let state = match state.unwrap_or(TaskStateArg::Done) {
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
        Commands::Gc => Ok(CommandRequest::Gc),
        Commands::Completion { .. } | Commands::CompleteTasks => Err(TaskspaceError::Internal(
            "completion command should be handled before parse".to_string(),
        )),
    }
}
