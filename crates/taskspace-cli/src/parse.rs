use taskspace_core::{SessionName, TaskspaceError};

use crate::Commands;
use crate::execute::CommandRequest;

pub fn parse_command(command: Commands) -> Result<CommandRequest, TaskspaceError> {
    match command {
        Commands::New {
            name,
            template,
            open,
            editors,
        } => Ok(CommandRequest::New {
            name: SessionName::parse(&name)?,
            template_path: template.map(std::path::PathBuf::from),
            open_after_create: open,
            editors_explicit: !editors.is_empty(),
            editors,
        }),
        Commands::Open {
            name,
            editors,
            last,
        } => {
            if name.is_some() && last {
                return Err(TaskspaceError::Usage(
                    "cannot use <NAME> with --last".to_string(),
                ));
            }
            let parsed_name = name.as_deref().map(SessionName::parse).transpose()?;
            Ok(CommandRequest::Open {
                name: parsed_name,
                editors_explicit: !editors.is_empty(),
                editors,
            })
        }
        Commands::List => Ok(CommandRequest::List),
        Commands::Rm { name, yes, dry_run } => Ok(CommandRequest::Remove {
            name: SessionName::parse(&name)?,
            yes,
            dry_run,
        }),
        Commands::Archive { name } => Ok(CommandRequest::Archive {
            name: SessionName::parse(&name)?,
        }),
        Commands::Doctor => Ok(CommandRequest::Doctor),
        Commands::Completion { shell } => Ok(CommandRequest::Completion { shell }),
        Commands::CompleteSessions => Ok(CommandRequest::CompleteSessions),
    }
}
