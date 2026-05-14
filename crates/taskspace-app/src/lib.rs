use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::Utc;
use taskspace_core::{SlotId, TaskspaceError, WorkspaceContext, WorkspaceSlot};
use taskspace_infra_fs::{create_dir, list_directories, run_command, run_command_in_dir};

const DEFAULT_AGENT: &str = "codex";
const DEFAULT_SLOT_COUNT: u16 = 2;

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    workspace_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InitWorkspacesResult {
    pub slots: Vec<WorkspaceSlot>,
}

#[derive(Debug, Clone)]
pub struct CheckoutResult {
    pub slot: WorkspaceSlot,
    pub git_ref: String,
}

#[derive(Debug, Clone)]
pub struct EnterSlotResult {
    pub agent: String,
    pub cwd: PathBuf,
    pub slot_id: String,
}

#[derive(Debug, Clone)]
pub struct HookContextResult {
    pub path: PathBuf,
    pub content: String,
}

impl TaskspaceApp {
    pub fn new(workspace_root: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            workspace_root: workspace_root.unwrap_or(default_workspace_root()?),
        })
    }

    pub fn init_workspaces(
        &self,
        source: &str,
        slot_count: Option<u16>,
    ) -> Result<InitWorkspacesResult> {
        let source = normalize_source(source)?;
        let slots = slot_count.unwrap_or(DEFAULT_SLOT_COUNT);
        if slots == 0 {
            return Err(anyhow!(TaskspaceError::Usage(
                "slot count must be greater than zero".to_string()
            )));
        }

        ensure_layout(&self.workspace_root)?;
        if !self.list_slots()?.is_empty() {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "workspace root already has slots under {}",
                self.slots_dir().display()
            ))));
        }

        let mut created = Vec::new();
        for index in 1..=slots {
            let slot_id = SlotId::parse(&format!("agent-{index}"))?;
            let slot_path = self.workspaces_dir().join(slot_id.as_str());
            git_clone(&source, &slot_path)?;

            let slot = WorkspaceSlot {
                id: slot_id,
                source: source.clone(),
                path: slot_path.clone(),
                last_checkout: current_git_head(&slot_path).ok(),
                updated_at: Utc::now().to_rfc3339(),
            };
            self.save_slot(&slot)?;
            self.write_workspace_context(&slot)?;
            created.push(slot);
        }

        Ok(InitWorkspacesResult { slots: created })
    }

    pub fn list_slots(&self) -> Result<Vec<WorkspaceSlot>> {
        ensure_layout(&self.workspace_root)?;
        let mut out = Vec::new();
        for entry in list_directories(&self.slots_dir()).map_err(map_infra_error)? {
            out.push(self.load_slot_by_id(entry.as_str())?);
        }
        out.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(out)
    }

    pub fn show_slot(&self, slot_ref: &str) -> Result<WorkspaceSlot> {
        self.load_slot_from_ref(slot_ref)
    }

    pub fn checkout(&self, slot_ref: &str, git_ref: &str) -> Result<CheckoutResult> {
        if git_ref.trim().is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(
                "git ref cannot be empty".to_string()
            )));
        }

        let mut slot = self.load_slot_from_ref(slot_ref)?;
        let slot_path = slot.path.clone();
        git_checkout(&slot_path, git_ref)?;

        slot.last_checkout = Some(git_ref.to_string());
        slot.updated_at = Utc::now().to_rfc3339();
        self.save_slot(&slot)?;
        self.write_workspace_context(&slot)?;

        Ok(CheckoutResult {
            slot,
            git_ref: git_ref.to_string(),
        })
    }

    pub fn enter_slot(&self, slot_ref: &str, agent: Option<&str>) -> Result<EnterSlotResult> {
        let slot = self.load_slot_from_ref(slot_ref)?;
        self.write_workspace_context(&slot)?;
        let agent = agent.unwrap_or(DEFAULT_AGENT);
        launch_agent(agent, &slot.path)?;

        Ok(EnterSlotResult {
            agent: agent.to_string(),
            cwd: slot.path,
            slot_id: slot.id.as_str().to_string(),
        })
    }

    pub fn hook_context(&self, start: Option<PathBuf>) -> Result<HookContextResult> {
        let start = match start {
            Some(path) => path,
            None => std::env::current_dir()
                .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?,
        };
        let context_path = find_workspace_context(&start).ok_or_else(|| {
            anyhow!(TaskspaceError::NotFound(format!(
                "no .taskspace/context.yaml found from {}",
                start.display()
            )))
        })?;
        let content = fs::read_to_string(&context_path)
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        let context: WorkspaceContext = serde_yaml::from_str(&content)
            .map_err(|err| anyhow!(TaskspaceError::Corrupt(err.to_string())))?;
        context.validate()?;

        Ok(HookContextResult {
            path: context_path,
            content,
        })
    }

    fn load_slot_from_ref(&self, slot_ref: &str) -> Result<WorkspaceSlot> {
        let slot_id = SlotId::parse(slot_ref)?;
        self.load_slot_by_id(slot_id.as_str())
    }

    fn load_slot_by_id(&self, id: &str) -> Result<WorkspaceSlot> {
        let slot_id = SlotId::parse(id)?;
        let slot_yaml = self.slots_dir().join(id).join("slot.yaml");
        if !slot_yaml.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "slot '{}' does not exist",
                id
            ))));
        }
        let raw = fs::read_to_string(&slot_yaml)
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        let slot: WorkspaceSlot = serde_yaml::from_str(&raw)
            .map_err(|err| anyhow!(TaskspaceError::Corrupt(err.to_string())))?;
        slot.validate()?;
        if slot.id.as_str() != slot_id.as_str() {
            return Err(anyhow!(TaskspaceError::Corrupt(format!(
                "slot id mismatch in registry entry: dir={} file={}",
                slot_id.as_str(),
                slot.id.as_str()
            ))));
        }
        Ok(slot)
    }

    fn save_slot(&self, slot: &WorkspaceSlot) -> Result<()> {
        slot.validate()?;
        let slot_dir = self.slots_dir().join(slot.id.as_str());
        create_dir(&slot_dir).map_err(map_infra_error)?;
        let yaml = serde_yaml::to_string(slot)
            .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
        let temp_path = slot_dir.join("slot.yaml.tmp");
        fs::write(&temp_path, yaml).map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        fs::rename(temp_path, slot_dir.join("slot.yaml"))
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
            .map(|_| ())
    }

    fn write_workspace_context(&self, slot: &WorkspaceSlot) -> Result<()> {
        let slot_path = &slot.path;
        let taskspace_dir = slot_path.join(".taskspace");
        create_dir(&taskspace_dir).map_err(map_infra_error)?;
        let context = WorkspaceContext::new(slot.clone());
        context.validate()?;
        let yaml = serde_yaml::to_string(&context)
            .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
        fs::write(taskspace_dir.join("context.yaml"), yaml)
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
    }

    fn workspaces_dir(&self) -> PathBuf {
        self.workspace_root.join("workspaces")
    }

    fn slots_dir(&self) -> PathBuf {
        self.workspace_root.join("state").join("slots")
    }
}

fn ensure_layout(workspace_root: &Path) -> Result<()> {
    create_dir(workspace_root).map_err(map_infra_error)?;
    create_dir(&workspace_root.join("workspaces")).map_err(map_infra_error)?;
    create_dir(&workspace_root.join("state").join("slots")).map_err(map_infra_error)?;
    Ok(())
}

fn default_workspace_root() -> Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(home.join("taskspace"))
}

fn normalize_source(raw: &str) -> Result<String> {
    if raw.trim().is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(
            "source cannot be empty".to_string()
        )));
    }
    let path = PathBuf::from(raw);
    if path.exists() {
        return fs::canonicalize(path)
            .map(|path| path.display().to_string())
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())));
    }
    Ok(raw.to_string())
}

fn current_git_head(slot_path: &Path) -> Result<String> {
    let branch = taskspace_infra_fs::run_command_capture(
        "git",
        &[
            "-C".to_string(),
            slot_path.display().to_string(),
            "branch".to_string(),
            "--show-current".to_string(),
        ],
    )?;
    if branch.is_empty() {
        taskspace_infra_fs::run_command_capture(
            "git",
            &[
                "-C".to_string(),
                slot_path.display().to_string(),
                "rev-parse".to_string(),
                "--short".to_string(),
                "HEAD".to_string(),
            ],
        )
    } else {
        Ok(branch)
    }
}

fn git_clone(source: &str, destination: &Path) -> Result<()> {
    run_command(
        "git",
        &[
            "clone".to_string(),
            source.to_string(),
            destination.display().to_string(),
        ],
    )
    .map_err(|err| {
        anyhow!(TaskspaceError::ExternalCommand(format!(
            "failed to clone source into {}: {err}",
            destination.display()
        )))
    })
}

fn git_checkout(repo: &Path, git_ref: &str) -> Result<()> {
    run_command(
        "git",
        &[
            "-C".to_string(),
            repo.display().to_string(),
            "checkout".to_string(),
            git_ref.to_string(),
        ],
    )
    .map_err(|err| {
        anyhow!(TaskspaceError::ExternalCommand(format!(
            "failed to checkout '{}' in {}: {err}",
            git_ref,
            repo.display()
        )))
    })
}

fn find_workspace_context(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };

    loop {
        let candidate = current.join(".taskspace").join("context.yaml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn launch_agent(agent: &str, cwd: &Path) -> Result<()> {
    match agent {
        "codex" => run_command_in_dir("codex", &[], cwd).map_err(|err| {
            anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to launch codex: {err}"
            )))
        }),
        "opencode" => run_command("opencode", &[cwd.display().to_string()]).map_err(|err| {
            anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to launch opencode: {err}"
            )))
        }),
        _ => Err(anyhow!(TaskspaceError::Usage(format!(
            "unsupported agent: {}",
            agent
        )))),
    }
}

fn map_infra_error(err: anyhow::Error) -> anyhow::Error {
    anyhow!(TaskspaceError::Io(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn init_git_repo(path: &Path) {
        fs::create_dir_all(path).expect("repo dir");
        run_command("git", &["init".to_string(), path.display().to_string()]).expect("git init");
        fs::write(path.join("README.md"), "demo\n").expect("readme");
        run_command(
            "git",
            &[
                "-C".to_string(),
                path.display().to_string(),
                "add".to_string(),
                "README.md".to_string(),
            ],
        )
        .expect("git add");
        run_command(
            "git",
            &[
                "-C".to_string(),
                path.display().to_string(),
                "-c".to_string(),
                "user.name=Test".to_string(),
                "-c".to_string(),
                "user.email=test@example.com".to_string(),
                "commit".to_string(),
                "-m".to_string(),
                "initial".to_string(),
            ],
        )
        .expect("git commit");
    }

    #[test]
    fn init_workspaces_clones_reusable_slots() {
        let temp = tempdir().expect("temp");
        let source = temp.path().join("source");
        init_git_repo(&source);
        let root = temp.path().join("taskspace");
        let app = TaskspaceApp::new(Some(root)).expect("app");

        let result = app
            .init_workspaces(&source.display().to_string(), Some(2))
            .expect("init");

        assert_eq!(result.slots.len(), 2);
        assert!(PathBuf::from(&result.slots[0].path).join(".git").exists());
        assert!(
            PathBuf::from(&result.slots[0].path)
                .join(".taskspace")
                .join("context.yaml")
                .exists()
        );
    }

    #[test]
    fn init_rejects_empty_slot_count_and_existing_slots() {
        let temp = tempdir().expect("temp");
        let source = temp.path().join("source");
        init_git_repo(&source);
        let root = temp.path().join("taskspace");
        let app = TaskspaceApp::new(Some(root)).expect("app");

        let err = app
            .init_workspaces(&source.display().to_string(), Some(0))
            .expect_err("zero slots should fail");
        assert!(err.to_string().contains("slot count"));

        app.init_workspaces(&source.display().to_string(), Some(1))
            .expect("init");

        let err = app
            .init_workspaces(&source.display().to_string(), Some(1))
            .expect_err("existing slots should fail");
        assert!(err.to_string().contains("already has slots"));
    }

    #[test]
    fn list_show_checkout_and_hook_context_work() {
        let temp = tempdir().expect("temp");
        let source = temp.path().join("source");
        init_git_repo(&source);
        let root = temp.path().join("taskspace");
        let app = TaskspaceApp::new(Some(root)).expect("app");

        app.init_workspaces(&source.display().to_string(), Some(1))
            .expect("init");

        let slots = app.list_slots().expect("list");
        assert_eq!(slots[0].id.as_str(), "agent-1");

        let shown = app.show_slot("agent-1").expect("show");
        assert_eq!(shown.id.as_str(), "agent-1");

        let checked_out = app.checkout("agent-1", "HEAD").expect("checkout");
        assert_eq!(checked_out.slot.last_checkout, Some("HEAD".to_string()));

        let context = app
            .hook_context(Some(checked_out.slot.path.clone()))
            .expect("context");
        assert!(context.content.contains("schema_version: 1"));
        assert!(context.content.contains("agent-1"));
    }
}
