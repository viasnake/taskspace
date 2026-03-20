use taskspace_app::DoctorLevel;

use crate::execute::CommandResult;

pub fn render(result: CommandResult) -> Vec<String> {
    match result {
        CommandResult::None => Vec::new(),
        CommandResult::Created(path) => vec![format!("created session: {}", path.display())],
        CommandResult::SessionList(list) => {
            if list.is_empty() {
                vec!["no sessions found".to_string()]
            } else {
                list
            }
        }
        CommandResult::Removed { name, dry_run } => {
            if dry_run {
                vec![format!("dry-run: session '{name}' can be removed")]
            } else {
                vec![format!("removed session: {name}")]
            }
        }
        CommandResult::Archived(path) => vec![format!("archived session to: {}", path.display())],
        CommandResult::Doctor(report) => report
            .checks
            .into_iter()
            .map(|check| {
                let label = match check.level {
                    DoctorLevel::Ok => "OK",
                    DoctorLevel::Warn => "WARN",
                    DoctorLevel::Fail => "FAIL",
                };
                format!("[{label}] {}", check.message)
            })
            .collect(),
    }
}
