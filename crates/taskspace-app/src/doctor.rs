use std::path::Path;

use anyhow::Result;
use taskspace_infra_fs::run_command_capture;

use crate::config::EditorRegistry;
use crate::paths::{archive_root, global_skills_paths};
use crate::spec;
use crate::validation::{validate_opencode_config, validate_workspace_yaml};
use crate::{DoctorCategory, DoctorCheck, DoctorLevel, DoctorReport, TaskspaceApp};

pub fn run(app: &TaskspaceApp, registry: &EditorRegistry) -> Result<DoctorReport> {
    let mut checks = Vec::new();

    if app.root_dir().exists() {
        checks.push(DoctorCheck {
            category: DoctorCategory::Filesystem,
            level: DoctorLevel::Ok,
            message: format!("taskspace root exists: {}", app.root_dir().display()),
        });
    } else {
        checks.push(DoctorCheck {
            category: DoctorCategory::Filesystem,
            level: DoctorLevel::Warn,
            message: format!(
                "taskspace root does not exist yet (it will be created on first new): {}",
                app.root_dir().display()
            ),
        });
    }

    let skills_paths = global_skills_paths()?;
    if skills_paths.iter().any(|path| path.exists()) {
        checks.push(DoctorCheck {
            category: DoctorCategory::Filesystem,
            level: DoctorLevel::Ok,
            message: "global SKILLS.md found".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
            category: DoctorCategory::Filesystem,
            level: DoctorLevel::Warn,
            message: "global SKILLS.md not found (~/.taskspace/SKILLS.md or ~/.config/taskspace/SKILLS.md)"
                .to_string(),
        });
    }

    if app.root_dir().exists() {
        for session in app.list_sessions()? {
            let session_dir = app.root_dir().join(&session);
            checks.extend(check_session(&session, &session_dir));
        }
    }

    let archive_root = archive_root(app.root_dir())?;
    if archive_root.exists() {
        checks.push(DoctorCheck {
            category: DoctorCategory::Filesystem,
            level: DoctorLevel::Ok,
            message: format!("archive directory exists: {}", archive_root.display()),
        });
    } else {
        checks.push(DoctorCheck {
            category: DoctorCategory::Filesystem,
            level: DoctorLevel::Warn,
            message: format!(
                "archive directory does not exist yet (it will be created on first archive): {}",
                archive_root.display()
            ),
        });
    }

    // Check git availability
    let git_level = if command_is_available("git") {
        DoctorLevel::Ok
    } else {
        DoctorLevel::Warn
    };
    checks.push(DoctorCheck {
        category: DoctorCategory::Command,
        level: git_level,
        message: "command check: git".to_string(),
    });

    // Check all editor commands from registry
    for (name, cmd) in editor_commands_for_check(registry) {
        let level = if command_is_available(&cmd) {
            DoctorLevel::Ok
        } else {
            DoctorLevel::Warn
        };
        checks.push(DoctorCheck {
            category: DoctorCategory::Command,
            level,
            message: format!("editor check: {} ({})", name, cmd),
        });
    }

    Ok(DoctorReport { checks })
}

fn editor_commands_for_check(registry: &EditorRegistry) -> Vec<(String, String)> {
    let mut editors: Vec<(String, String)> = registry
        .all_editors()
        .filter_map(|(name, config)| {
            config
                .command
                .first()
                .map(|cmd| (name.to_string(), cmd.to_string()))
        })
        .collect();
    editors.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    editors
}

fn command_is_available(command: &str) -> bool {
    for probe in ["--version", "-V", "version"] {
        let args = vec![probe.to_string()];
        if run_command_capture(command, &args).is_ok() {
            return true;
        }
    }
    false
}

fn check_session(name: &str, session_dir: &Path) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let mut missing = Vec::new();
    for rel in spec::REQUIRED_SESSION_FILES {
        if !session_dir.join(rel).exists() {
            missing.push(rel.to_string());
        }
    }

    if !missing.is_empty() {
        checks.push(DoctorCheck {
            category: DoctorCategory::Session,
            level: DoctorLevel::Fail,
            message: format!("session '{}' missing: {}", name, missing.join(", ")),
        });
        return checks;
    }

    checks.push(DoctorCheck {
        category: DoctorCategory::Session,
        level: DoctorLevel::Ok,
        message: format!("session '{}' structure looks valid", name),
    });

    checks.push(
        match validate_workspace_yaml(&session_dir.join("workspace.yaml")) {
            Ok(()) => DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Ok,
                message: format!("session '{}' workspace.yaml is valid", name),
            },
            Err(err) => DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Fail,
                message: format!("session '{}' workspace.yaml invalid: {err}", name),
            },
        },
    );

    checks.push(
        match validate_opencode_config(&session_dir.join(".opencode/opencode.jsonc")) {
            Ok(()) => DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Ok,
                message: format!("session '{}' opencode instructions are valid", name),
            },
            Err(err) => DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Fail,
                message: format!("session '{}' opencode config invalid: {err}", name),
            },
        },
    );

    checks
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn editor_commands_for_check_returns_sorted_names() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
[editors.zzz]
command = ["zzz"]

[editors.aaa]
command = ["aaa"]
"#,
        )
        .expect("write config");

        let registry = EditorRegistry::load_from(Some(&config_path)).expect("registry");
        let names: Vec<String> = editor_commands_for_check(&registry)
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn command_is_available_reports_false_for_missing_command() {
        assert!(!command_is_available("definitely-not-existing-command-xyz"));
    }
}
