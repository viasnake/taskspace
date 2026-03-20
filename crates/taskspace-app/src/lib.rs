mod doctor;
mod paths;
mod repo_import;
mod spec;
mod template;
mod validation;

use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::Utc;
use taskspace_core::{EditorKind, RepoSpec, SessionName, TaskspaceError};
use taskspace_infra_fs::{create_dir, list_directories, move_dir, remove_dir_all, run_command};

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    root_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct NewSessionRequest {
    pub name: SessionName,
    pub repos: Vec<RepoSpec>,
    pub open_after_create: bool,
    pub editor: EditorKind,
}

#[derive(Debug, Clone)]
pub struct OpenSessionRequest {
    pub name: SessionName,
    pub editor: EditorKind,
}

#[derive(Debug, Clone)]
pub struct RemoveSessionRequest {
    pub name: SessionName,
    pub yes: bool,
    pub dry_run: bool,
}

#[derive(Debug, Clone)]
pub struct ArchiveSessionRequest {
    pub name: SessionName,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone)]
pub struct DoctorCheck {
    pub level: DoctorLevel,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum DoctorLevel {
    Ok,
    Warn,
    Fail,
}

impl TaskspaceApp {
    pub fn new(root_dir: Option<PathBuf>) -> Result<Self> {
        let resolved = match root_dir {
            Some(path) => path,
            None => paths::default_sessions_root()?,
        };
        Ok(Self { root_dir: resolved })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn create_session(&self, request: NewSessionRequest) -> Result<PathBuf> {
        create_dir(&self.root_dir).map_err(map_infra_error)?;

        let session_dir = self.root_dir.join(request.name.as_str());
        if session_dir.exists() {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "session '{}' already exists",
                request.name.as_str()
            ))));
        }

        create_dir(&session_dir).map_err(map_infra_error)?;
        if let Err(err) = (|| -> Result<()> {
            template::create_base_structure(&session_dir)?;
            template::write_templates(&session_dir, request.name.as_str(), &request.repos)?;
            repo_import::import_repos(&session_dir, &request.repos)?;
            Ok(())
        })() {
            if let Err(cleanup_err) = remove_dir_all(&session_dir).map_err(map_infra_error) {
                return Err(anyhow!(TaskspaceError::Io(format!(
                    "failed to rollback session directory: {cleanup_err}; original error: {err}"
                ))));
            }
            return Err(err);
        }

        if request.open_after_create {
            self.open_session(OpenSessionRequest {
                name: request.name,
                editor: request.editor,
            })?;
        }

        Ok(session_dir)
    }

    pub fn list_sessions(&self) -> Result<Vec<String>> {
        list_directories(&self.root_dir).map_err(map_infra_error)
    }

    pub fn open_session(&self, request: OpenSessionRequest) -> Result<()> {
        let session_dir = self.root_dir.join(request.name.as_str());
        if !session_dir.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "session '{}' does not exist",
                request.name.as_str()
            ))));
        }

        let target = match request.editor {
            EditorKind::Code => session_dir.join("workspace.code-workspace"),
            EditorKind::Opencode => session_dir,
        };
        let arg = vec![target.display().to_string()];
        let command = match request.editor {
            EditorKind::Code => "code",
            EditorKind::Opencode => "opencode",
        };

        run_command(command, &arg).map_err(|err| {
            anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to open session '{}': {err}",
                request.name.as_str()
            )))
        })
    }

    pub fn doctor(&self) -> Result<DoctorReport> {
        doctor::run(self)
    }

    pub fn archive_session(&self, request: ArchiveSessionRequest) -> Result<PathBuf> {
        let session_dir = self.root_dir.join(request.name.as_str());
        if !session_dir.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "session '{}' does not exist",
                request.name.as_str()
            ))));
        }

        validation::ensure_session_marker(&session_dir)?;
        let archive_root = paths::archive_root(&self.root_dir)?;
        create_dir(&archive_root).map_err(map_infra_error)?;

        let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
        let destination = archive_root.join(format!("{}-{timestamp}", request.name.as_str()));
        if destination.exists() {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "archive destination already exists: {}",
                destination.display()
            ))));
        }

        move_dir(&session_dir, &destination).map_err(map_infra_error)?;
        Ok(destination)
    }

    pub fn remove_session(&self, request: RemoveSessionRequest) -> Result<()> {
        let session_dir = self.root_dir.join(request.name.as_str());
        if !session_dir.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "session '{}' does not exist",
                request.name.as_str()
            ))));
        }

        validation::ensure_session_marker(&session_dir)?;

        if request.dry_run {
            return Ok(());
        }
        if !request.yes {
            return Err(anyhow!(TaskspaceError::Usage(
                "rm requires --yes for destructive operation".to_string(),
            )));
        }

        remove_dir_all(&session_dir).map_err(map_infra_error)
    }
}

pub(crate) fn map_infra_error(err: anyhow::Error) -> anyhow::Error {
    anyhow!(TaskspaceError::Io(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn create_and_list_session() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            repos: vec![],
            open_after_create: false,
            editor: EditorKind::Opencode,
        })
        .expect("session creation");

        let sessions = app.list_sessions().expect("list sessions");
        assert_eq!(sessions, vec!["demo".to_string()]);
    }

    #[test]
    fn doctor_fails_when_required_file_missing() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            repos: vec![],
            open_after_create: false,
            editor: EditorKind::Opencode,
        })
        .expect("create");

        fs::remove_file(temp.path().join("demo/context/PLAN.md")).expect("remove plan");
        let report = app.doctor().expect("doctor");
        assert!(
            report
                .checks
                .iter()
                .any(|c| matches!(c.level, DoctorLevel::Fail))
        );
    }

    #[test]
    fn remove_requires_yes_unless_dry_run() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            repos: vec![],
            open_after_create: false,
            editor: EditorKind::Opencode,
        })
        .expect("session creation");

        let err = app
            .remove_session(RemoveSessionRequest {
                name: SessionName::parse("demo").expect("name"),
                yes: false,
                dry_run: false,
            })
            .expect_err("missing --yes");
        assert!(format!("{err}").contains("requires --yes"));

        app.remove_session(RemoveSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            yes: false,
            dry_run: true,
        })
        .expect("dry run");

        app.remove_session(RemoveSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            yes: true,
            dry_run: false,
        })
        .expect("remove");
    }
}
