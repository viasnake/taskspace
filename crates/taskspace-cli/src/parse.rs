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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finish_defaults_to_done() {
        let command = parse_command(Commands::Finish {
            task: "tsk_demo01".to_string(),
            state: None,
        })
        .expect("parsed");

        match command {
            CommandRequest::Finish { task_ref, state } => {
                assert_eq!(task_ref, "tsk_demo01");
                assert_eq!(state, TaskState::Done);
            }
            _ => panic!("unexpected command"),
        }
    }

    #[test]
    fn finish_maps_explicit_states() {
        let blocked = parse_command(Commands::Finish {
            task: "tsk_demo01".to_string(),
            state: Some(TaskStateArg::Blocked),
        })
        .expect("blocked");
        let review = parse_command(Commands::Finish {
            task: "tsk_demo01".to_string(),
            state: Some(TaskStateArg::Review),
        })
        .expect("review");
        let archived = parse_command(Commands::Finish {
            task: "tsk_demo01".to_string(),
            state: Some(TaskStateArg::Archived),
        })
        .expect("archived");

        assert!(matches!(
            blocked,
            CommandRequest::Finish {
                state: TaskState::Blocked,
                ..
            }
        ));
        assert!(matches!(
            review,
            CommandRequest::Finish {
                state: TaskState::Review,
                ..
            }
        ));
        assert!(matches!(
            archived,
            CommandRequest::Finish {
                state: TaskState::Archived,
                ..
            }
        ));
    }

    #[test]
    fn completion_commands_are_rejected() {
        assert!(parse_command(Commands::Completion { shell: None }).is_err());
        assert!(parse_command(Commands::CompleteTasks).is_err());
    }
}
