use taskspace_core::{EditorKind, RepoSpec, SessionName, TaskspaceError};

use crate::execute::CommandRequest;
use crate::{CliEditor, Commands};

pub fn parse_command(command: Commands) -> Result<CommandRequest, TaskspaceError> {
    match command {
        Commands::New {
            name,
            repos,
            open,
            editor,
        } => Ok(CommandRequest::New {
            name: SessionName::parse(&name)?,
            repos: repos
                .iter()
                .map(|raw| RepoSpec::parse(raw))
                .collect::<Result<Vec<_>, _>>()?,
            open_after_create: open,
            editor: editor.into(),
        }),
        Commands::Open { name, editor } => Ok(CommandRequest::Open {
            name: SessionName::parse(&name)?,
            editor: editor.into(),
        }),
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
    }
}

impl From<CliEditor> for EditorKind {
    fn from(value: CliEditor) -> Self {
        match value {
            CliEditor::Opencode => EditorKind::Opencode,
            CliEditor::Code => EditorKind::Code,
        }
    }
}
