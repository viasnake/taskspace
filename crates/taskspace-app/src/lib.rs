mod config;
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
use taskspace_core::{
    EditorConfig, PlaceholderContext, SessionName, TaskspaceError, expand_placeholders,
};
use taskspace_infra_fs::{
    create_dir, list_directories, list_directories_with_modified, move_dir, remove_dir_all,
    spawn_command,
};

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    root_dir: PathBuf,
    editor_registry: config::EditorRegistry,
}

#[derive(Debug, Clone)]
pub struct NewSessionRequest {
    pub name: SessionName,
    pub template_path: Option<PathBuf>,
    pub open_after_create: bool,
    pub editors: Vec<String>,
    pub editors_explicit: bool,
}

#[derive(Debug, Clone)]
pub struct OpenSessionRequest {
    pub target: OpenSessionTarget,
    pub editors: Vec<String>,
    pub editors_explicit: bool,
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
        let editor_registry = config::EditorRegistry::load()?;
        Ok(Self {
            root_dir: resolved,
            editor_registry,
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    pub fn create_session(&self, request: NewSessionRequest) -> Result<PathBuf> {
        if request.open_after_create {
            self.resolve_editor_configs(&request.editors, request.editors_explicit)?;
        }
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
                editors: request.editors,
                editors_explicit: request.editors_explicit,
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

        let editor_configs =
            self.resolve_editor_configs(&request.editors, request.editors_explicit)?;

        let mut failures = Vec::new();
        let mut skipped_unavailable = Vec::new();
        for (editor_name, editor_config) in editor_configs {
            if let Err(err) = launch_editor(editor_config, &session_dir) {
                if !request.editors_explicit && is_command_not_found(&err) {
                    skipped_unavailable.push(editor_name.to_string());
                    continue;
                }
                failures.push(format!("{editor_name}: {err:#}"));
            }
        }

        if failures.is_empty()
            && !(!request.editors_explicit && skipped_unavailable.len() == request.editors.len())
        {
            Ok(())
        } else {
            if !request.editors_explicit && skipped_unavailable.len() == request.editors.len() {
                return Err(anyhow!(TaskspaceError::ExternalCommand(format!(
                    "failed to open session '{}': no default editors are available (skipped: [{}])\nhint: run 'taskspace doctor' or specify --editor <name>",
                    session_name.as_str(),
                    skipped_unavailable.join(", ")
                ))));
            }
            Err(anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to open session '{}' with editors [{}]\nhint: run 'taskspace doctor' or specify --editor <name>",
                session_name.as_str(),
                failures.join("; ")
            ))))
        }
    }

    pub fn doctor(&self) -> Result<DoctorReport> {
        doctor::run(self, &self.editor_registry)
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
    fn resolve_editor_configs<'a>(
        &'a self,
        editors: &'a [String],
        editors_explicit: bool,
    ) -> Result<Vec<(&'a str, &'a EditorConfig)>> {
        if editors.is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(
                "at least one editor must be specified".to_string()
            )));
        }

        let mut resolved = Vec::new();
        for editor in editors {
            let editor_name = editor.as_str();
            let config = self.editor_registry.get(editor_name);
            match (config, editors_explicit) {
                (Some(config), _) => resolved.push((editor_name, config)),
                (None, false) => continue,
                (None, true) => {
                    return Err(anyhow!(TaskspaceError::Usage(format!(
                        "unknown editor: '{}'. Available editors: {}",
                        editor_name,
                        self.available_editor_names().join(", ")
                    ))));
                }
            }
        }

        if resolved.is_empty() {
            return Err(anyhow!(TaskspaceError::ExternalCommand(
                "failed to open session: no default editors are configured\nhint: run 'taskspace doctor' or specify --editor <name>"
                    .to_string()
            )));
        }

        Ok(resolved)
    }

    fn available_editor_names(&self) -> Vec<&str> {
        let mut names = self.editor_registry.editor_names().collect::<Vec<_>>();
        names.sort_unstable();
        names
    }

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

/// Launches an editor with the given configuration.
fn launch_editor(config: &EditorConfig, session_dir: &Path) -> Result<()> {
    let context = PlaceholderContext::new(session_dir);
    let expanded_cmd = expand_placeholders(&config.command, &context);

    let Some((command, args)) = expanded_cmd.split_first() else {
        return Err(anyhow!(TaskspaceError::Usage(
            "editor command cannot be empty".to_string()
        )));
    };
    if command.trim().is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(
            "editor executable cannot be empty".to_string()
        )));
    }

    spawn_command(command, args)
}

fn is_command_not_found(err: &anyhow::Error) -> bool {
    err.chain().any(|source| {
        source
            .downcast_ref::<std::io::Error>()
            .is_some_and(|io_err| io_err.kind() == std::io::ErrorKind::NotFound)
    })
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
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
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
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
        })
        .expect("session creation");

        let yaml =
            fs::read_to_string(temp.path().join("demo/workspace.yaml")).expect("workspace yaml");
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).expect("valid yaml");

        assert_eq!(value["version"].as_u64(), Some(4));
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
        assert!(
            value["manifest"]["projects"][0]["resolved_commit"]
                .as_str()
                .expect("resolved commit")
                .len()
                >= 40
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
                editors: vec!["opencode".to_string()],
                editors_explicit: true,
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
                editors: vec!["opencode".to_string()],
                editors_explicit: true,
            })
            .expect_err("clone failure should rollback");

        assert!(format!("{err}").contains("failed to clone project"));
        assert!(!temp.path().join("demo").exists());
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

    #[test]
    fn doctor_fails_when_required_file_missing() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
        })
        .expect("create");

        fs::remove_file(temp.path().join("demo/AGENTS.md")).expect("remove agents");
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
            template_path: None,
            open_after_create: false,
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
        })
        .expect("session creation");

        let err = app
            .remove_session(RemoveSessionRequest {
                name: SessionName::parse("demo").expect("name"),
                yes: false,
                dry_run: false,
            })
            .expect_err("missing --yes");
        assert!(format!("{err}").contains("without --yes"));

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

    #[test]
    fn open_last_fails_when_no_sessions_exist() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Last,
                editors: vec!["opencode".to_string()],
                editors_explicit: true,
            })
            .expect_err("open last without sessions should fail");
        assert!(format!("{err}").contains("no session specified and no recent session found"));
    }

    #[test]
    fn create_session_allows_unknown_editor_when_not_opening() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["definitely-not-an-editor".to_string()],
            editors_explicit: true,
        })
        .expect("unknown editor should be ignored when not opening");

        assert!(temp.path().join("demo").exists());
    }

    #[test]
    fn list_sessions_ignores_non_session_directories() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
        })
        .expect("create session");

        fs::create_dir_all(temp.path().join(".archive")).expect("create archive dir");

        let sessions = app.list_sessions().expect("list sessions");
        assert_eq!(sessions, vec!["demo".to_string()]);
    }

    #[test]
    fn open_session_fails_when_editor_list_is_empty() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
        })
        .expect("create session");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
                editors: Vec::new(),
                editors_explicit: true,
            })
            .expect_err("open should reject empty editors");

        assert!(format!("{err}").contains("at least one editor"));
    }

    #[test]
    fn open_session_fails_for_unknown_editor_name() {
        let temp = tempdir().expect("temp dir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["opencode".to_string()],
            editors_explicit: true,
        })
        .expect("create session");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
                editors: vec!["definitely-not-an-editor".to_string()],
                editors_explicit: true,
            })
            .expect_err("open should reject unknown editor");

        assert!(format!("{err}").contains("unknown editor"));
    }

    #[test]
    fn open_session_implicit_editors_skips_missing_executables() {
        let temp = tempdir().expect("temp dir");
        let config_path = temp.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
[editors.missing]
command = ["definitely-not-existing-command-xyz", "{dir}"]

[editors.available]
command = ["true"]
"#,
        )
        .expect("write config");

        let app = TaskspaceApp {
            root_dir: temp.path().to_path_buf(),
            editor_registry: config::EditorRegistry::load_from(Some(&config_path))
                .expect("registry"),
        };

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["available".to_string()],
            editors_explicit: true,
        })
        .expect("create session");

        app.open_session(OpenSessionRequest {
            target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
            editors: vec!["missing".to_string(), "available".to_string()],
            editors_explicit: false,
        })
        .expect("implicit open should skip unavailable editor");
    }

    #[test]
    fn open_session_implicit_editors_fail_when_all_unavailable() {
        let temp = tempdir().expect("temp dir");
        let config_path = temp.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
[editors.missing1]
command = ["definitely-not-existing-command-xyz"]

[editors.missing2]
command = ["definitely-not-existing-command-abc"]
"#,
        )
        .expect("write config");

        let app = TaskspaceApp {
            root_dir: temp.path().to_path_buf(),
            editor_registry: config::EditorRegistry::load_from(Some(&config_path))
                .expect("registry"),
        };

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["missing1".to_string()],
            editors_explicit: true,
        })
        .expect("create session");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
                editors: vec!["missing1".to_string(), "missing2".to_string()],
                editors_explicit: false,
            })
            .expect_err("implicit open should fail when all editors are unavailable");

        assert!(format!("{err}").contains("no default editors are available"));
        assert!(format!("{err}").contains("taskspace doctor"));
    }

    #[test]
    fn open_session_explicit_editor_failure_contains_os_error_details() {
        let temp = tempdir().expect("temp dir");
        let config_path = temp.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
[editors.missing]
command = ["definitely-not-existing-command-xyz"]
"#,
        )
        .expect("write config");

        let app = TaskspaceApp {
            root_dir: temp.path().to_path_buf(),
            editor_registry: config::EditorRegistry::load_from(Some(&config_path))
                .expect("registry"),
        };

        app.create_session(NewSessionRequest {
            name: SessionName::parse("demo").expect("name"),
            template_path: None,
            open_after_create: false,
            editors: vec!["missing".to_string()],
            editors_explicit: true,
        })
        .expect("create session");

        let err = app
            .open_session(OpenSessionRequest {
                target: OpenSessionTarget::Name(SessionName::parse("demo").expect("name")),
                editors: vec!["missing".to_string()],
                editors_explicit: true,
            })
            .expect_err("explicit open should fail for missing command");

        let rendered = format!("{err}");
        assert!(rendered.contains("failed to execute command"));
        assert!(rendered.contains("os error") || rendered.contains("No such file"));
    }
}
