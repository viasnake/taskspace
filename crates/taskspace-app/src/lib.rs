mod doctor;
mod paths;
mod repo_import;
mod spec;
mod template;
mod validation;

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Result, anyhow};
use chrono::Utc;
use taskspace_core::{SessionName, TaskspaceError};
use taskspace_infra_fs::{
    create_dir, list_directories, list_directories_with_modified, move_dir, remove_dir_all,
    run_command,
};

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    root_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct NewSessionRequest {
    pub name: SessionName,
    pub template_path: Option<PathBuf>,
    pub open_after_create: bool,
}

#[derive(Debug, Clone)]
pub struct OpenSessionRequest {
    pub target: OpenSessionTarget,
}

#[derive(Debug, Clone)]
pub enum OpenSessionTarget {
    Name(SessionName),
    Last,
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
    pub category: DoctorCategory,
    pub level: DoctorLevel,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorCategory {
    Filesystem,
    Session,
    Command,
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
            let mut workspace = template::resolve_workspace_model(
                request.name.as_str(),
                request.template_path.as_deref(),
            )?;
            if let Some(manifest) = &mut workspace.manifest {
                repo_import::clone_manifest_projects(&session_dir, manifest)?;
            }
            template::write_templates(&session_dir, &workspace)?;
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
                target: OpenSessionTarget::Name(request.name),
            })?;
        }

        Ok(session_dir)
    }

    pub fn list_sessions(&self) -> Result<Vec<String>> {
        let sessions = list_directories(&self.root_dir).map_err(map_infra_error)?;
        Ok(sessions
            .into_iter()
            .filter(|name| is_visible_session_name(name))
            .collect())
    }

    pub fn open_session(&self, request: OpenSessionRequest) -> Result<()> {
        let session_name = match request.target {
            OpenSessionTarget::Name(name) => name,
            OpenSessionTarget::Last => self.find_latest_session_name()?.ok_or_else(|| {
                anyhow!(TaskspaceError::NotFound(
                    "no session specified and no recent session found".to_string()
                ))
            })?,
        };

        let session_dir = self.root_dir.join(session_name.as_str());
        if !session_dir.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "session '{}' does not exist",
                session_name.as_str()
            ))));
        }

        let workspace = validation::validate_workspace_yaml(
            &session_dir.join("workspace.yaml"),
            session_name.as_str(),
        )?;

        let mut failures = Vec::new();
        for (index, action) in workspace.open.actions.iter().enumerate() {
            if let Err(err) = launch_open_action(&action.command, &session_dir) {
                failures.push(format!("action {}: {err:#}", index + 1));
            }
        }

        if failures.is_empty() {
            return Ok(());
        }

        Err(anyhow!(TaskspaceError::ExternalCommand(format!(
            "failed to open session '{}'\n{}",
            session_name.as_str(),
            failures.join("; ")
        ))))
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
            return Err(anyhow!(TaskspaceError::Usage(format!(
                "refusing to remove session '{}' without --yes\nhint: rerun with: taskspace rm {} --yes",
                request.name.as_str(),
                request.name.as_str()
            ))));
        }

        remove_dir_all(&session_dir).map_err(map_infra_error)
    }
}

impl TaskspaceApp {
    fn find_latest_session_name(&self) -> Result<Option<SessionName>> {
        let mut sessions: Vec<(SessionName, SystemTime)> =
            list_directories_with_modified(&self.root_dir)
                .map_err(map_infra_error)?
                .into_iter()
                .filter_map(|entry| {
                    parse_visible_session_name(&entry.name).map(|name| (name, entry.modified))
                })
                .collect();

        sessions.sort_by(|left, right| match right.1.cmp(&left.1) {
            Ordering::Equal => left.0.as_str().cmp(right.0.as_str()),
            non_eq => non_eq,
        });

        Ok(sessions.into_iter().next().map(|(name, _)| name))
    }
}

fn is_visible_session_name(name: &str) -> bool {
    !name.starts_with('.') && SessionName::parse(name).is_ok()
}

fn parse_visible_session_name(name: &str) -> Option<SessionName> {
    if name.starts_with('.') {
        return None;
    }

    SessionName::parse(name).ok()
}

fn launch_open_action(command_template: &[String], session_dir: &Path) -> Result<()> {
    if command_template.is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(
            "open action command cannot be empty".to_string()
        )));
    }

    let expanded = command_template
        .iter()
        .map(|arg| arg.replace("{dir}", &session_dir.display().to_string()))
        .collect::<Vec<_>>();
    let Some((program, args)) = expanded.split_first() else {
        return Err(anyhow!(TaskspaceError::Usage(
            "open action command cannot be empty".to_string()
        )));
    };
    if program.trim().is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(
            "open action executable cannot be empty".to_string()
        )));
    }

    run_command(program, args)
}

pub(crate) fn map_infra_error(err: anyhow::Error) -> anyhow::Error {
    anyhow!(TaskspaceError::Io(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    #[test]
    fn create_and_list_session() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
        })
        .expect("session creation");

        let sessions = app.list_sessions().expect("list sessions");
        assert_eq!(sessions, vec!["demo".to_string()]);
    }

    #[test]
    fn create_session_with_template_records_template_metadata() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let repo = create_git_repo(temp.path(), "seed-repo");

        let template_path = temp.path().join("template.yaml");
        fs::write(
            &template_path,
            format!(
                "version: 1\nmanifest:\n  projects:\n    - id: app\n      source: {}\n      revision: main\n      target: repos/app\n",
                repo.display()
            ),
        )
        .expect("write template");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: Some(template_path.clone()),
            open_after_create: false,
        })
        .expect("session creation");

        let yaml =
            fs::read_to_string(temp.path().join("demo/workspace.yaml")).expect("workspace yaml");
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).expect("valid yaml");

        assert_eq!(value["version"].as_u64(), Some(5));
        assert_eq!(value["created_by"].as_str(), Some("template"));
        let expected_ref = path_to_string(&template_path);
        assert_eq!(
            value["template"]["ref"].as_str(),
            Some(expected_ref.as_str())
        );
        assert!(
            value["template"]["digest"]
                .as_str()
                .expect("digest")
                .starts_with("sha256:")
        );
        assert_eq!(value["manifest"]["projects"][0]["id"].as_str(), Some("app"));
        assert_eq!(
            value["open"]["actions"][0]["command"][0].as_str(),
            Some("opencode")
        );
        assert!(temp.path().join("demo/repos/app/README.md").exists());
    }

    #[test]
    fn create_session_fails_for_invalid_template_and_rolls_back() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let template_path = temp.path().join("invalid-template.yaml");
        fs::write(&template_path, "version: 99\n").expect("write template");

        let err = app
            .create_session(NewSessionRequest {
                name: SessionName::parse("demo").expect("name"),
                template_path: Some(template_path),
                open_after_create: false,
            })
            .expect_err("invalid template should fail");

        assert!(format!("{err}").contains("unsupported template schema version"));
        assert!(!temp.path().join("demo").exists());
    }

    #[test]
    fn create_session_rolls_back_when_clone_fails() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let missing_repo = temp.path().join("does-not-exist");
        let template_path = temp.path().join("template.yaml");
        fs::write(
            &template_path,
            format!(
                "version: 1\nmanifest:\n  projects:\n    - id: bad\n      source: {}\n      target: repos/bad\n",
                missing_repo.display()
            ),
        )
        .expect("write template");

        let err = app
            .create_session(NewSessionRequest {
                name: SessionName::parse("demo").expect("name"),
                template_path: Some(template_path),
                open_after_create: false,
            })
            .expect_err("clone failure should rollback");

        assert!(format!("{err}").contains("failed to clone project"));
        assert!(!temp.path().join("demo").exists());
    }

    #[test]
    fn open_last_fails_when_no_sessions_exist() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Last,
            })
            .expect_err("open last without sessions should fail");
        assert!(format!("{err}").contains("no session specified and no recent session found"));
    }

    #[test]
    fn open_session_fails_when_action_command_fails() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
        })
        .expect("create session");

        fs::write(
            temp.path().join("demo/workspace.yaml"),
            "version: 5\nname: demo\ncreated_at: 2026-01-01T00:00:00Z\nlayout_version: 1\ncreated_by: manual\nopen:\n  actions:\n    - command: [\"false\"]\n",
        )
        .expect("write workspace");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
            })
            .expect_err("open should fail");

        assert!(format!("{err}").contains("failed to open session 'demo'"));
    }

    #[test]
    fn open_session_succeeds_when_actions_succeed() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
        })
        .expect("create session");

        fs::write(
            temp.path().join("demo/workspace.yaml"),
            "version: 5\nname: demo\ncreated_at: 2026-01-01T00:00:00Z\nlayout_version: 1\ncreated_by: manual\nopen:\n  actions:\n    - command: [\"true\"]\n    - command: [\"true\"]\n",
        )
        .expect("write workspace");

        app.open_session(OpenSessionRequest {
            target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
        })
        .expect("open should succeed");
    }

    #[test]
    fn launch_open_action_expands_dir_placeholder() {
        let temp = tempdir().expect("temp dir");
        let marker = temp.path().join("ran.txt");
        let command = vec![
            "sh".to_string(),
            "-c".to_string(),
            "test -d \"$1\" && touch \"$1/ran.txt\"".to_string(),
            "sh".to_string(),
            "{dir}".to_string(),
        ];

        launch_open_action(&command, temp.path()).expect("open action should run");
        assert!(marker.exists());
    }

    fn path_to_string(path: &Path) -> String {
        path.display().to_string()
    }

    fn create_git_repo(base: &Path, name: &str) -> std::path::PathBuf {
        let repo = base.join(name);
        fs::create_dir_all(&repo).expect("create repo dir");
        run_git(&repo, &["init", "-b", "main"]);
        fs::write(repo.join("README.md"), "seed repo\n").expect("write readme");
        run_git(&repo, &["add", "README.md"]);
        run_git(
            &repo,
            &[
                "-c",
                "user.name=taskspace",
                "-c",
                "user.email=taskspace@example.com",
                "commit",
                "-m",
                "initial",
            ],
        );
        repo
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {:?}", args);
    }
}
