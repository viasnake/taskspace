use taskspace_app::{DoctorCategory, DoctorLevel};

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
        CommandResult::Doctor(report) => {
            let mut lines = Vec::new();
            let categories = [
                DoctorCategory::Filesystem,
                DoctorCategory::Session,
                DoctorCategory::Command,
            ];

            for category in categories {
                let section_checks: Vec<_> = report
                    .checks
                    .iter()
                    .filter(|check| check.category == category)
                    .collect();
                if section_checks.is_empty() {
                    continue;
                }

                lines.push(format!("{}:", category_label(category)));
                lines.extend(section_checks.into_iter().map(|check| {
                    let label = match check.level {
                        DoctorLevel::Ok => "OK",
                        DoctorLevel::Warn => "WARN",
                        DoctorLevel::Fail => "FAIL",
                    };
                    format!("[{label}] {}", check.message)
                }));
            }

            lines
        }
    }
}

fn category_label(category: DoctorCategory) -> &'static str {
    match category {
        DoctorCategory::Filesystem => "filesystem",
        DoctorCategory::Session => "session",
        DoctorCategory::Command => "command",
    }
}
