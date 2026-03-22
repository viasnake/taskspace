use std::path::Path;

#[cfg(windows)]
use std::path::PathBuf;

use anyhow::Result;
use taskspace_infra_fs::run_command_capture;

use crate::spec;
use crate::template::WorkspaceModel;
use crate::validation::validate_workspace_yaml;
use crate::{DoctorCategory, DoctorCheck, DoctorLevel, DoctorReport, TaskspaceApp};

pub fn run(app: &TaskspaceApp) -> Result<DoctorReport> {
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

    if app.root_dir().exists() {
        for session in app.list_sessions()? {
            let session_dir = app.root_dir().join(&session);
            checks.extend(check_session(&session, &session_dir));
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

    let workspace = match validate_workspace_yaml(&session_dir.join("workspace.yaml"), name) {
        Ok(workspace) => {
            checks.push(DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Ok,
                message: format!("session '{}' workspace.yaml is valid", name),
            });
            Some(workspace)
        }
        Err(err) => {
            checks.push(DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Fail,
                message: format!("session '{}' workspace.yaml invalid: {err}", name),
            });
            None
        }
    };

    if let Some(workspace) = workspace {
        checks.extend(check_template_metadata(name, session_dir, &workspace));
        checks.extend(check_open_actions(name, &workspace));
    }

    checks
}

fn check_template_metadata(
    name: &str,
    session_dir: &Path,
    workspace: &WorkspaceModel,
) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    if let Some(template) = &workspace.template
        && !Path::new(&template.ref_path).exists()
    {
        checks.push(DoctorCheck {
            category: DoctorCategory::Session,
            level: DoctorLevel::Warn,
            message: format!(
                "session '{}' template reference does not exist: {}",
                name, template.ref_path
            ),
        });
    }

    if let Some(manifest) = &workspace.manifest {
        let missing = manifest
            .projects
            .iter()
            .filter(|project| !session_dir.join(&project.target).exists())
            .map(|project| format!("{} -> {}", project.id, project.target))
            .collect::<Vec<_>>();

        if !missing.is_empty() {
            checks.push(DoctorCheck {
                category: DoctorCategory::Session,
                level: DoctorLevel::Warn,
                message: format!(
                    "session '{}' manifest targets are missing: {}",
                    name,
                    missing.join(", ")
                ),
            });
        }

        for project in &manifest.projects {
            let project_path = session_dir.join(&project.target);
            if !project_path.exists() {
                continue;
            }

            let args = vec![
                "-C".to_string(),
                project_path.display().to_string(),
                "rev-parse".to_string(),
                "HEAD".to_string(),
            ];
            match run_command_capture("git", &args) {
                Ok(current) => {
                    if let Some(expected) = &project.resolved_commit
                        && expected != &current
                    {
                        checks.push(DoctorCheck {
                            category: DoctorCategory::Session,
                            level: DoctorLevel::Warn,
                            message: format!(
                                "session '{}' manifest project '{}' HEAD differs (expected {}, got {})",
                                name, project.id, expected, current
                            ),
                        });
                    }
                }
                Err(err) => {
                    checks.push(DoctorCheck {
                        category: DoctorCategory::Session,
                        level: DoctorLevel::Warn,
                        message: format!(
                            "session '{}' manifest project '{}' is not a readable git repository: {}",
                            name, project.id, err
                        ),
                    });
                }
            }
        }
    }

    checks
}

fn check_open_actions(name: &str, workspace: &WorkspaceModel) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    for action in &workspace.open.actions {
        if let Some(program) = action.command.first() {
            let level = if command_is_available(program) {
                DoctorLevel::Ok
            } else {
                DoctorLevel::Warn
            };
            checks.push(DoctorCheck {
                category: DoctorCategory::Command,
                level,
                message: format!("session '{}' open action command check: {}", name, program),
            });
        }
    }

    checks
}

fn command_is_available(command: &str) -> bool {
    if command.trim().is_empty() {
        return false;
    }

    let command_path = Path::new(command);
    if command_path.is_absolute() || command.contains(std::path::MAIN_SEPARATOR) {
        return command_path.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };

    for dir in std::env::split_paths(&path_var) {
        if command_exists_in_dir(&dir, command) {
            return true;
        }
    }

    false
}

fn command_exists_in_dir(dir: &Path, command: &str) -> bool {
    let candidate = dir.join(command);
    if candidate.is_file() {
        return true;
    }

    #[cfg(windows)]
    {
        let pathext = std::env::var_os("PATHEXT").unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
        let exts = pathext.to_string_lossy();
        for ext in exts.split(';') {
            if ext.is_empty() {
                continue;
            }
            let ext = ext.trim_start_matches('.');
            let with_ext = PathBuf::from(format!("{}.{}", candidate.display(), ext));
            if with_ext.is_file() {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_is_available_reports_false_for_missing_command() {
        assert!(!command_is_available("definitely-not-existing-command-xyz"));
    }
}
