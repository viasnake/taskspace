use std::path::Path;

use anyhow::Result;
use taskspace_infra_fs::run_command_capture;

use crate::config::EditorRegistry;
use crate::paths::{archive_root, global_skills_paths};
use crate::spec;
use crate::validation::{validate_opencode_config, validate_workspace_yaml};
use crate::{DoctorCheck, DoctorLevel, DoctorReport, TaskspaceApp};

pub fn run(app: &TaskspaceApp, registry: &EditorRegistry) -> Result<DoctorReport> {
    let mut checks = Vec::new();

    if app.root_dir().exists() {
        checks.push(DoctorCheck {
            level: DoctorLevel::Ok,
            message: format!("taskspace root exists: {}", app.root_dir().display()),
        });
    } else {
        checks.push(DoctorCheck {
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
            level: DoctorLevel::Ok,
            message: "global SKILLS.md found".to_string(),
        });
    } else {
        checks.push(DoctorCheck {
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
            level: DoctorLevel::Ok,
            message: format!("archive directory exists: {}", archive_root.display()),
        });
    } else {
        checks.push(DoctorCheck {
            level: DoctorLevel::Warn,
            message: format!(
                "archive directory does not exist yet (it will be created on first archive): {}",
                archive_root.display()
            ),
        });
    }

    // Check git availability
    let git_level = if run_command_capture("git", &["--version".to_string()]).is_ok() {
        DoctorLevel::Ok
    } else {
        DoctorLevel::Warn
    };
    checks.push(DoctorCheck {
        level: git_level,
        message: "command check: git".to_string(),
    });

    // Check all editor commands from registry
    for (name, config) in registry.all_editors() {
        if let Some(cmd_str) = config.command.first() {
            let level = if run_command_capture(cmd_str, &["--version".to_string()]).is_ok() {
                DoctorLevel::Ok
            } else {
                DoctorLevel::Warn
            };
            checks.push(DoctorCheck {
                level,
                message: format!("editor check: {} ({})", name, cmd_str),
            });
        }
    }

    Ok(DoctorReport { checks })
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
            level: DoctorLevel::Fail,
            message: format!("session '{}' missing: {}", name, missing.join(", ")),
        });
        return checks;
    }

    checks.push(DoctorCheck {
        level: DoctorLevel::Ok,
        message: format!("session '{}' structure looks valid", name),
    });

    checks.push(
        match validate_workspace_yaml(&session_dir.join("workspace.yaml")) {
            Ok(()) => DoctorCheck {
                level: DoctorLevel::Ok,
                message: format!("session '{}' workspace.yaml is valid", name),
            },
            Err(err) => DoctorCheck {
                level: DoctorLevel::Fail,
                message: format!("session '{}' workspace.yaml invalid: {err}", name),
            },
        },
    );

    checks.push(
        match validate_opencode_config(&session_dir.join(".opencode/opencode.jsonc")) {
            Ok(()) => DoctorCheck {
                level: DoctorLevel::Ok,
                message: format!("session '{}' opencode instructions are valid", name),
            },
            Err(err) => DoctorCheck {
                level: DoctorLevel::Fail,
                message: format!("session '{}' opencode config invalid: {err}", name),
            },
        },
    );

    checks
}
