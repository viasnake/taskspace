use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use chrono::Utc;
use taskspace_core::{
    Project, ProjectId, SlotId, SlotRef, TaskspaceError, WorkspaceContext, WorkspaceSlot,
};
use taskspace_infra_fs::{
    create_dir, list_directories, read_file, remove_dir_all, run_command, run_command_capture,
    run_command_in_dir,
};

const DEFAULT_AGENT: &str = "codex";

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    workspace_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InitWorkspaceResult {
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AddProjectResult {
    pub project: Project,
}

#[derive(Debug, Clone)]
pub struct AddSlotsResult {
    pub project: Project,
    pub slots: Vec<WorkspaceSlot>,
}

#[derive(Debug, Clone)]
pub struct RemoveSlotResult {
    pub slot: WorkspaceSlot,
}

#[derive(Debug, Clone)]
pub struct EnterSlotResult {
    pub agent: String,
    pub cwd: PathBuf,
    pub slot_ref: SlotRef,
}

#[derive(Debug, Clone)]
pub struct HookContextResult {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct SyncSlotStatus {
    pub slot: WorkspaceSlot,
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct SyncSlotsResult {
    pub statuses: Vec<SyncSlotStatus>,
}

impl SyncSlotsResult {
    pub fn has_failures(&self) -> bool {
        self.statuses.iter().any(|status| !status.success)
    }
}

impl TaskspaceApp {
    pub fn new(workspace_root: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            workspace_root: workspace_root.unwrap_or(default_workspace_root()?),
        })
    }

    pub fn init_workspace(&self) -> Result<InitWorkspaceResult> {
        self.ensure_ready_layout()?;
        Ok(InitWorkspaceResult {
            root: self.workspace_root.clone(),
        })
    }

    pub fn add_project(&self, project_ref: &str, source: &str) -> Result<AddProjectResult> {
        self.ensure_ready_layout()?;
        let project_id = ProjectId::parse(project_ref)?;
        let project_yaml = self.project_yaml_path(project_id.as_str());
        if project_yaml.exists() {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "project '{}' already exists",
                project_id.as_str()
            ))));
        }

        let project = Project {
            id: project_id,
            source: normalize_source(source)?,
            updated_at: now_rfc3339(),
        };
        self.save_project(&project)?;
        Ok(AddProjectResult { project })
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        self.ensure_ready_layout()?;
        let mut out = Vec::new();
        for entry in list_directories(&self.projects_dir()).map_err(map_infra_error)? {
            out.push(self.load_project(&entry)?);
        }
        out.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(out)
    }

    pub fn show_project(&self, project_ref: &str) -> Result<Project> {
        self.ensure_ready_layout()?;
        self.load_project(project_ref)
    }

    pub fn add_slots(&self, project_ref: &str, count: Option<u16>) -> Result<AddSlotsResult> {
        self.ensure_ready_layout()?;
        let project = self.load_project(project_ref)?;
        let count = count.unwrap_or(1);
        if count == 0 {
            return Err(anyhow!(TaskspaceError::Usage(
                "slot count must be greater than zero".to_string(),
            )));
        }

        let mut slots = self.list_slots_for_project(project.id.as_str())?;
        let mut created = Vec::new();
        for _ in 0..count {
            let slot_id = next_slot_id(&slots)?;
            let slot_path = self
                .workspaces_dir()
                .join(project.id.as_str())
                .join(slot_id.as_str());
            git_clone(&project.source, &slot_path)?;
            let slot = WorkspaceSlot {
                project_id: project.id.clone(),
                id: slot_id,
                path: slot_path,
                last_sync_at: None,
                updated_at: now_rfc3339(),
            };
            self.save_slot(&slot)?;
            self.write_workspace_context(&project, &slot)?;
            slots.push(slot.clone());
            created.push(slot);
        }

        Ok(AddSlotsResult {
            project,
            slots: created,
        })
    }

    pub fn list_slots(&self) -> Result<Vec<WorkspaceSlot>> {
        self.ensure_ready_layout()?;
        let mut out = Vec::new();
        for project in self.list_projects()? {
            out.extend(self.list_slots_for_project(project.id.as_str())?);
        }
        out.sort_by(|a, b| {
            a.project_id
                .as_str()
                .cmp(b.project_id.as_str())
                .then_with(|| a.id.as_str().cmp(b.id.as_str()))
        });
        Ok(out)
    }

    pub fn list_slots_for_project(&self, project_ref: &str) -> Result<Vec<WorkspaceSlot>> {
        self.ensure_ready_layout()?;
        let project_id = ProjectId::parse(project_ref)?;
        let slots_dir = self.project_slots_dir(project_id.as_str());
        let mut out = Vec::new();
        for entry in list_directories(&slots_dir).map_err(map_infra_error)? {
            out.push(self.load_slot_by_parts(project_id.as_str(), &entry)?);
        }
        out.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        Ok(out)
    }

    pub fn show_slot(&self, slot_ref: &str) -> Result<WorkspaceSlot> {
        self.ensure_ready_layout()?;
        self.load_slot(slot_ref)
    }

    pub fn remove_slot(&self, slot_ref: &str, force: bool) -> Result<RemoveSlotResult> {
        self.ensure_ready_layout()?;
        let slot = self.load_slot(slot_ref)?;
        if !force && git_is_dirty(&slot.path)? {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "slot '{}' has uncommitted changes; rerun with --force",
                slot.slot_ref().as_string()
            ))));
        }

        let slot_yaml = self.slot_yaml_path(slot.project_id.as_str(), slot.id.as_str());
        if slot_yaml.exists() {
            fs::remove_file(&slot_yaml)
                .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        }
        let slot_state_dir = self
            .project_slots_dir(slot.project_id.as_str())
            .join(slot.id.as_str());
        if slot_state_dir.exists() {
            remove_dir_all(&slot_state_dir).map_err(map_infra_error)?;
        }
        if slot.path.exists() {
            remove_dir_all(&slot.path).map_err(map_infra_error)?;
        }

        Ok(RemoveSlotResult { slot })
    }

    pub fn sync_project(&self, project_ref: &str) -> Result<SyncSlotsResult> {
        self.ensure_ready_layout()?;
        let slots = self.list_slots_for_project(project_ref)?;
        self.sync_loaded_slots(slots)
    }

    pub fn sync_all(&self) -> Result<SyncSlotsResult> {
        self.ensure_ready_layout()?;
        let slots = self.list_slots()?;
        self.sync_loaded_slots(slots)
    }

    pub fn enter_slot(
        &self,
        slot_ref: &str,
        agent: Option<&str>,
        sync_before_enter: bool,
    ) -> Result<EnterSlotResult> {
        self.ensure_ready_layout()?;
        let mut slot = self.load_slot(slot_ref)?;
        let project = self.load_project(slot.project_id.as_str())?;

        if sync_before_enter {
            git_fetch_all_prune(&slot.path)?;
            slot.last_sync_at = Some(now_rfc3339());
            slot.updated_at = now_rfc3339();
            self.save_slot(&slot)?;
        }

        self.write_workspace_context(&project, &slot)?;
        let agent = agent.unwrap_or(DEFAULT_AGENT);
        launch_agent(agent, &slot.path)?;
        let slot_ref = slot.slot_ref();
        let cwd = slot.path.clone();

        Ok(EnterSlotResult {
            agent: agent.to_string(),
            cwd,
            slot_ref,
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

    fn sync_loaded_slots(&self, slots: Vec<WorkspaceSlot>) -> Result<SyncSlotsResult> {
        let mut statuses = Vec::new();
        for mut slot in slots {
            match git_fetch_all_prune(&slot.path) {
                Ok(()) => {
                    slot.last_sync_at = Some(now_rfc3339());
                    slot.updated_at = now_rfc3339();
                    self.save_slot(&slot)?;
                    let project = self.load_project(slot.project_id.as_str())?;
                    self.write_workspace_context(&project, &slot)?;
                    statuses.push(SyncSlotStatus {
                        slot,
                        success: true,
                        message: "fetched".to_string(),
                    });
                }
                Err(err) => statuses.push(SyncSlotStatus {
                    slot,
                    success: false,
                    message: err.to_string(),
                }),
            }
        }
        Ok(SyncSlotsResult { statuses })
    }

    fn load_project(&self, project_ref: &str) -> Result<Project> {
        let project_id = ProjectId::parse(project_ref)?;
        let raw =
            read_file(&self.project_yaml_path(project_id.as_str())).map_err(map_infra_error)?;
        let project: Project = serde_yaml::from_str(&raw)
            .map_err(|err| anyhow!(TaskspaceError::Corrupt(err.to_string())))?;
        project.validate()?;
        if project.id.as_str() != project_id.as_str() {
            return Err(anyhow!(TaskspaceError::Corrupt(format!(
                "project id mismatch in registry entry: dir={} file={}",
                project_id.as_str(),
                project.id.as_str()
            ))));
        }
        Ok(project)
    }

    fn save_project(&self, project: &Project) -> Result<()> {
        project.validate()?;
        let project_dir = self.projects_dir().join(project.id.as_str());
        create_dir(&project_dir).map_err(map_infra_error)?;
        write_yaml_atomically(&project_dir.join("project.yaml"), project)
    }

    fn load_slot(&self, slot_ref: &str) -> Result<WorkspaceSlot> {
        let slot_ref = SlotRef::parse(slot_ref)?;
        self.load_slot_by_parts(slot_ref.project_id().as_str(), slot_ref.slot_id().as_str())
    }

    fn load_slot_by_parts(&self, project_ref: &str, slot_ref: &str) -> Result<WorkspaceSlot> {
        let project_id = ProjectId::parse(project_ref)?;
        let slot_id = SlotId::parse(slot_ref)?;
        let raw = read_file(&self.slot_yaml_path(project_id.as_str(), slot_id.as_str()))
            .map_err(map_infra_error)?;
        let slot: WorkspaceSlot = serde_yaml::from_str(&raw)
            .map_err(|err| anyhow!(TaskspaceError::Corrupt(err.to_string())))?;
        slot.validate()?;
        if slot.project_id.as_str() != project_id.as_str() || slot.id.as_str() != slot_id.as_str() {
            return Err(anyhow!(TaskspaceError::Corrupt(format!(
                "slot id mismatch in registry entry: project={} slot={}",
                project_id.as_str(),
                slot_id.as_str()
            ))));
        }
        Ok(slot)
    }

    fn save_slot(&self, slot: &WorkspaceSlot) -> Result<()> {
        slot.validate()?;
        let slot_dir = self
            .project_slots_dir(slot.project_id.as_str())
            .join(slot.id.as_str());
        create_dir(&slot_dir).map_err(map_infra_error)?;
        write_yaml_atomically(&slot_dir.join("slot.yaml"), slot)
    }

    fn write_workspace_context(&self, project: &Project, slot: &WorkspaceSlot) -> Result<()> {
        let taskspace_dir = slot.path.join(".taskspace");
        create_dir(&taskspace_dir).map_err(map_infra_error)?;
        let context = WorkspaceContext::new(project.clone(), slot.clone());
        context.validate()?;
        let yaml = serde_yaml::to_string(&context)
            .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
        fs::write(taskspace_dir.join("context.yaml"), yaml)
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
    }

    fn ensure_ready_layout(&self) -> Result<()> {
        detect_legacy_layout(&self.workspace_root)?;
        create_dir(&self.workspace_root).map_err(map_infra_error)?;
        create_dir(&self.workspaces_dir()).map_err(map_infra_error)?;
        create_dir(&self.state_dir()).map_err(map_infra_error)?;
        create_dir(&self.projects_dir()).map_err(map_infra_error)?;
        Ok(())
    }

    fn workspaces_dir(&self) -> PathBuf {
        self.workspace_root.join("workspaces")
    }

    fn state_dir(&self) -> PathBuf {
        self.workspace_root.join("state")
    }

    fn projects_dir(&self) -> PathBuf {
        self.state_dir().join("projects")
    }

    fn project_yaml_path(&self, project_ref: &str) -> PathBuf {
        self.projects_dir().join(project_ref).join("project.yaml")
    }

    fn project_slots_dir(&self, project_ref: &str) -> PathBuf {
        self.projects_dir().join(project_ref).join("slots")
    }

    fn slot_yaml_path(&self, project_ref: &str, slot_ref: &str) -> PathBuf {
        self.project_slots_dir(project_ref)
            .join(slot_ref)
            .join("slot.yaml")
    }
}

fn detect_legacy_layout(workspace_root: &Path) -> Result<()> {
    let legacy_slots = workspace_root.join("state").join("slots");
    if !legacy_slots.exists() {
        return Ok(());
    }

    let entries = list_directories(&legacy_slots).map_err(map_infra_error)?;
    if entries.is_empty() {
        return Ok(());
    }

    Err(anyhow!(TaskspaceError::Conflict(format!(
        "legacy slot layout detected under {}; use a new --root",
        legacy_slots.display()
    ))))
}

fn default_workspace_root() -> Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(home.join("taskspace"))
}

fn normalize_source(raw: &str) -> Result<String> {
    if raw.trim().is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(
            "source cannot be empty".to_string(),
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

fn next_slot_id(existing: &[WorkspaceSlot]) -> Result<SlotId> {
    let mut used = std::collections::BTreeSet::new();
    for slot in existing {
        if let Some(index) = slot.id.as_str().strip_prefix("agent-")
            && let Ok(index) = index.parse::<u16>()
        {
            used.insert(index);
        }
    }

    let mut next = 1;
    while used.contains(&next) {
        next += 1;
    }
    SlotId::parse(&format!("agent-{next}")).map_err(anyhow::Error::from)
}

fn write_yaml_atomically<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let yaml = serde_yaml::to_string(value)
        .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, yaml).map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
    fs::rename(temp_path, path).map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
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

fn git_fetch_all_prune(repo: &Path) -> Result<()> {
    run_command(
        "git",
        &[
            "-C".to_string(),
            repo.display().to_string(),
            "fetch".to_string(),
            "--all".to_string(),
            "--prune".to_string(),
        ],
    )
    .map_err(|err| {
        anyhow!(TaskspaceError::ExternalCommand(format!(
            "failed to fetch in {}: {err}",
            repo.display()
        )))
    })
}

fn git_is_dirty(repo: &Path) -> Result<bool> {
    let out = run_command_capture(
        "git",
        &[
            "-C".to_string(),
            repo.display().to_string(),
            "status".to_string(),
            "--porcelain".to_string(),
        ],
    )
    .map_err(|err| {
        anyhow!(TaskspaceError::ExternalCommand(format!(
            "failed to inspect git status in {}: {err}",
            repo.display()
        )))
    })?;
    Ok(!out.is_empty())
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

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
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
    fn init_and_manage_projects_and_slots() {
        let temp = tempdir().expect("temp");
        let source = temp.path().join("source");
        init_git_repo(&source);
        let root = temp.path().join("taskspace");
        let app = TaskspaceApp::new(Some(root.clone())).expect("app");

        app.init_workspace().expect("init");
        app.add_project("app", &source.display().to_string())
            .expect("project");

        let added = app.add_slots("app", Some(2)).expect("slot add");
        assert_eq!(added.slots.len(), 2);
        assert!(added.slots[0].path.join(".git").exists());
        assert!(
            added.slots[0]
                .path
                .join(".taskspace")
                .join("context.yaml")
                .exists()
        );

        let projects = app.list_projects().expect("projects");
        assert_eq!(projects[0].id.as_str(), "app");

        let slots = app.list_slots_for_project("app").expect("slots");
        assert_eq!(slots[0].id.as_str(), "agent-1");
        assert_eq!(slots[1].id.as_str(), "agent-2");
        assert_eq!(
            root.join("workspaces").join("app").join("agent-1"),
            slots[0].path
        );
    }

    #[test]
    fn slot_remove_respects_dirty_state_and_force() {
        let temp = tempdir().expect("temp");
        let source = temp.path().join("source");
        init_git_repo(&source);
        let root = temp.path().join("taskspace");
        let app = TaskspaceApp::new(Some(root)).expect("app");

        app.init_workspace().expect("init");
        app.add_project("app", &source.display().to_string())
            .expect("project");
        let slot = app
            .add_slots("app", Some(1))
            .expect("slot add")
            .slots
            .remove(0);

        fs::write(slot.path.join("README.md"), "changed\n").expect("change");
        let err = app
            .remove_slot("app:agent-1", false)
            .expect_err("dirty slot should fail");
        assert!(err.to_string().contains("--force"));

        let removed = app.remove_slot("app:agent-1", true).expect("force remove");
        assert_eq!(removed.slot.id.as_str(), "agent-1");
        assert!(!removed.slot.path.exists());
    }

    #[test]
    fn sync_updates_context_and_last_sync_at() {
        let temp = tempdir().expect("temp");
        let source = temp.path().join("source");
        init_git_repo(&source);
        let root = temp.path().join("taskspace");
        let app = TaskspaceApp::new(Some(root)).expect("app");

        app.init_workspace().expect("init");
        app.add_project("app", &source.display().to_string())
            .expect("project");
        let slot = app
            .add_slots("app", Some(1))
            .expect("slot add")
            .slots
            .remove(0);

        let result = app.sync_project("app").expect("sync");
        assert_eq!(result.statuses.len(), 1);
        assert!(result.statuses[0].success);

        let shown = app.show_slot("app:agent-1").expect("show");
        assert!(shown.last_sync_at.is_some());

        let context = app.hook_context(Some(slot.path)).expect("context");
        assert!(context.content.contains("schema_version: 2"));
        assert!(context.content.contains("project:"));
        assert!(context.content.contains("last_sync_at:"));
    }

    #[test]
    fn legacy_layout_is_rejected() {
        let temp = tempdir().expect("temp");
        let root = temp.path().join("taskspace");
        fs::create_dir_all(root.join("state").join("slots").join("agent-1")).expect("legacy");
        let app = TaskspaceApp::new(Some(root)).expect("app");
        let err = app.init_workspace().expect_err("legacy should fail");
        assert!(err.to_string().contains("legacy slot layout"));
    }
}
