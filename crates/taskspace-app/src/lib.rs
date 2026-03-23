use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use chrono::Utc;
use taskspace_core::{
    Root, RootAccess, RootIsolation, RootType, Task, TaskId, TaskState, TaskspaceError, VerifySpec,
};
use taskspace_infra_fs::{create_dir, list_directories, remove_dir_all, run_command};

const DEFAULT_ADAPTER: &str = "opencode";

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    state_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StartTaskRequest {
    pub title: String,
    pub entry_adapter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttachRootRequest {
    pub task_ref: String,
    pub root_type: RootType,
    pub path: PathBuf,
    pub role: String,
    pub access: RootAccess,
    pub isolation: RootIsolation,
}

#[derive(Debug, Clone)]
pub struct DetachRootRequest {
    pub task_ref: String,
    pub root_id: String,
}

#[derive(Debug, Clone)]
pub struct EnterTaskRequest {
    pub task_ref: String,
    pub adapter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VerifyTaskRequest {
    pub task_ref: String,
}

#[derive(Debug, Clone)]
pub struct FinishTaskRequest {
    pub task_ref: String,
    pub target_state: TaskState,
}

#[derive(Debug, Clone)]
pub struct ArchiveTaskRequest {
    pub task_ref: String,
}

#[derive(Debug, Clone)]
pub struct ShowTaskRequest {
    pub task_ref: String,
}

#[derive(Debug, Clone)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub state: TaskState,
    pub roots_count: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct AttachRootResult {
    pub root_id: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnterTaskResult {
    pub adapter: String,
    pub cwd: PathBuf,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct VerifyTaskResult {
    pub task_id: String,
    pub ran: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GcResult {
    pub removed: Vec<PathBuf>,
}

impl TaskspaceApp {
    pub fn new(state_root: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            state_root: state_root.unwrap_or(default_state_root()?),
        })
    }

    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    pub fn start_task(&self, request: StartTaskRequest) -> Result<Task> {
        if request.title.trim().is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(
                "task title cannot be empty".to_string()
            )));
        }

        ensure_layout(&self.state_root)?;

        let id = new_task_id()?;
        let task_id = TaskId::parse(&id)?;
        let now = Utc::now().to_rfc3339();
        let scratch_path = self.scratch_dir().join(task_id.as_str());
        create_dir(&scratch_path).map_err(map_infra_error)?;

        let task = Task {
            id: task_id,
            title: request.title.clone(),
            slug: slugify(&request.title),
            state: TaskState::Active,
            updated_at: now,
            entry_adapter: request
                .entry_adapter
                .unwrap_or_else(|| DEFAULT_ADAPTER.to_string()),
            roots: vec![Root {
                id: "root_scratch".to_string(),
                root_type: RootType::Scratch,
                path: scratch_path.display().to_string(),
                role: "scratch".to_string(),
                access: RootAccess::Rw,
                isolation: RootIsolation::Generated,
                branch: None,
                base_branch: None,
                include: Vec::new(),
                exclude: Vec::new(),
            }],
            verify: VerifySpec::default(),
            notes: Default::default(),
        };
        task.validate()?;
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskSummary>> {
        ensure_layout(&self.state_root)?;
        let mut out = Vec::new();
        for entry in list_directories(&self.registry_tasks_dir()).map_err(map_infra_error)? {
            let task = self.load_task_by_id(entry.as_str())?;
            out.push(TaskSummary {
                id: task.id.as_str().to_string(),
                title: task.title,
                state: task.state,
                roots_count: task.roots.len(),
                updated_at: task.updated_at,
            });
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(out)
    }

    pub fn show_task(&self, request: ShowTaskRequest) -> Result<Task> {
        let task = self.load_task_from_ref(request.task_ref.as_str())?;
        Ok(task)
    }

    pub fn attach_root(&self, request: AttachRootRequest) -> Result<AttachRootResult> {
        let mut task = self.load_task_from_ref(request.task_ref.as_str())?;
        let canonical = fs::canonicalize(&request.path).unwrap_or(request.path.clone());
        if !canonical.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "root path does not exist: {}",
                canonical.display()
            ))));
        }

        let root_id = next_root_id(&task.roots);
        let root = Root {
            id: root_id.clone(),
            root_type: request.root_type,
            path: canonical.display().to_string(),
            role: request.role,
            access: request.access,
            isolation: request.isolation,
            branch: None,
            base_branch: None,
            include: Vec::new(),
            exclude: Vec::new(),
        };
        root.validate()?;
        task.roots.push(root);
        task.updated_at = Utc::now().to_rfc3339();

        let warnings = self.collect_rw_direct_warnings(&task)?;
        self.save_task(&task)?;
        Ok(AttachRootResult { root_id, warnings })
    }

    pub fn detach_root(&self, request: DetachRootRequest) -> Result<()> {
        let mut task = self.load_task_from_ref(request.task_ref.as_str())?;
        let before = task.roots.len();
        task.roots.retain(|root| root.id != request.root_id);
        if task.roots.len() == before {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "root id not found: {}",
                request.root_id
            ))));
        }
        task.updated_at = Utc::now().to_rfc3339();
        self.save_task(&task)
    }

    pub fn verify_task(&self, request: VerifyTaskRequest) -> Result<VerifyTaskResult> {
        let task = self.load_task_from_ref(request.task_ref.as_str())?;
        let cwd = self.resolve_default_cwd(&task)?;
        let mut ran = Vec::new();
        for command in &task.verify.commands {
            let (program, args) = parse_verify_command(command)?;
            let status = Command::new(program)
                .args(args)
                .current_dir(&cwd)
                .status()
                .map_err(|err| {
                    anyhow!(TaskspaceError::ExternalCommand(format!(
                        "failed to execute verify command '{}': {}",
                        command, err
                    )))
                })?;
            if !status.success() {
                return Err(anyhow!(TaskspaceError::ExternalCommand(format!(
                    "verify command failed: '{}' (status: {})",
                    command, status
                ))));
            }
            ran.push(command.clone());
        }
        Ok(VerifyTaskResult {
            task_id: task.id.as_str().to_string(),
            ran,
        })
    }

    pub fn enter_task(&self, request: EnterTaskRequest) -> Result<EnterTaskResult> {
        let task = self.load_task_from_ref(request.task_ref.as_str())?;
        let adapter = request
            .adapter
            .unwrap_or_else(|| task.entry_adapter.clone());
        let view_dir = self.prepare_synthesized_view(&task)?;
        launch_adapter(adapter.as_str(), &view_dir)?;

        Ok(EnterTaskResult {
            adapter,
            cwd: view_dir,
            task_id: task.id.as_str().to_string(),
        })
    }

    pub fn finish_task(&self, request: FinishTaskRequest) -> Result<TaskState> {
        let mut task = self.load_task_from_ref(request.task_ref.as_str())?;
        if !task.state.can_transition_to(request.target_state) {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "cannot transition task from {:?} to {:?}",
                task.state, request.target_state
            ))));
        }
        task.state = request.target_state;
        task.updated_at = Utc::now().to_rfc3339();
        self.save_task(&task)?;
        Ok(task.state)
    }

    pub fn archive_task(&self, request: ArchiveTaskRequest) -> Result<()> {
        let mut task = self.load_task_from_ref(request.task_ref.as_str())?;
        if !task.state.can_transition_to(TaskState::Archived) {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "cannot archive task from state {:?}",
                task.state
            ))));
        }
        task.state = TaskState::Archived;
        task.updated_at = Utc::now().to_rfc3339();
        self.save_task(&task)
    }

    pub fn gc(&self) -> Result<GcResult> {
        ensure_layout(&self.state_root)?;
        let mut active_ids = HashSet::new();
        for summary in self.list_tasks()? {
            active_ids.insert(summary.id);
        }

        let mut removed = Vec::new();
        for name in list_directories(&self.scratch_dir()).map_err(map_infra_error)? {
            if !active_ids.contains(&name) {
                let path = self.scratch_dir().join(&name);
                remove_dir_all(&path).map_err(map_infra_error)?;
                removed.push(path);
            }
        }
        for name in list_directories(&self.views_dir()).map_err(map_infra_error)? {
            if !active_ids.contains(&name) {
                let path = self.views_dir().join(&name);
                remove_dir_all(&path).map_err(map_infra_error)?;
                removed.push(path);
            }
        }

        Ok(GcResult { removed })
    }

    fn collect_rw_direct_warnings(&self, task: &Task) -> Result<Vec<String>> {
        let mut warnings = Vec::new();
        let summaries = self.list_tasks()?;
        let shared_path_candidates: Vec<_> = task
            .roots
            .iter()
            .filter(|root| {
                root.access == RootAccess::Rw
                    && root.isolation == RootIsolation::Direct
                    && matches!(root.root_type, RootType::Git | RootType::Artifact)
            })
            .map(|root| root.path.clone())
            .collect();

        if shared_path_candidates.is_empty() {
            return Ok(warnings);
        }

        for summary in summaries {
            if summary.id == task.id.as_str() {
                continue;
            }
            let other = self.load_task_by_id(summary.id.as_str())?;
            for root in &other.roots {
                if root.access == RootAccess::Rw
                    && root.isolation == RootIsolation::Direct
                    && shared_path_candidates.contains(&root.path)
                {
                    warnings.push(format!(
                        "shared rw direct root detected with task {} at {}",
                        other.id.as_str(),
                        root.path
                    ));
                }
            }
        }

        Ok(warnings)
    }

    fn load_task_from_ref(&self, task_ref: &str) -> Result<Task> {
        if task_ref == "current" {
            let current = self.resolve_current_task_id()?;
            return self.load_task_by_id(current.as_str());
        }
        let task_id = TaskId::parse(task_ref)?;
        self.load_task_by_id(task_id.as_str())
    }

    fn resolve_current_task_id(&self) -> Result<String> {
        let tasks = self.list_tasks()?;
        let current = tasks
            .iter()
            .find(|item| {
                matches!(
                    item.state,
                    TaskState::Active | TaskState::Blocked | TaskState::Review
                )
            })
            .or_else(|| tasks.first())
            .ok_or_else(|| anyhow!(TaskspaceError::NotFound("no task found".to_string())))?;
        Ok(current.id.clone())
    }

    fn load_task_by_id(&self, id: &str) -> Result<Task> {
        let task_id = TaskId::parse(id)?;
        let task_yaml = self.registry_tasks_dir().join(id).join("task.yaml");
        if !task_yaml.exists() {
            return Err(anyhow!(TaskspaceError::NotFound(format!(
                "task '{}' does not exist",
                id
            ))));
        }
        let raw = fs::read_to_string(&task_yaml)
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        let task: Task = serde_yaml::from_str(&raw)
            .map_err(|err| anyhow!(TaskspaceError::Corrupt(err.to_string())))?;
        task.validate()?;
        if task.id.as_str() != task_id.as_str() {
            return Err(anyhow!(TaskspaceError::Corrupt(format!(
                "task id mismatch in registry entry: dir={} file={}",
                task_id.as_str(),
                task.id.as_str()
            ))));
        }
        Ok(task)
    }

    fn save_task(&self, task: &Task) -> Result<()> {
        task.validate()?;
        let task_dir = self.registry_tasks_dir().join(task.id.as_str());
        create_dir(&task_dir).map_err(map_infra_error)?;
        let yaml = serde_yaml::to_string(task)
            .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
        let temp_path = task_dir.join("task.yaml.tmp");
        fs::write(&temp_path, yaml).map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        fs::rename(temp_path, task_dir.join("task.yaml"))
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
            .map(|_| ())
    }

    fn resolve_default_cwd(&self, task: &Task) -> Result<PathBuf> {
        for root in &task.roots {
            if matches!(root.root_type, RootType::Git | RootType::Dir)
                && root.access == RootAccess::Rw
            {
                return Ok(PathBuf::from(&root.path));
            }
        }
        for root in &task.roots {
            let path = PathBuf::from(&root.path);
            if path.is_dir() {
                return Ok(path);
            }
        }
        Err(anyhow!(TaskspaceError::NotFound(
            "no suitable working directory found for task".to_string(),
        )))
    }

    fn prepare_synthesized_view(&self, task: &Task) -> Result<PathBuf> {
        let view_dir = self.views_dir().join(task.id.as_str());
        create_dir(&view_dir).map_err(map_infra_error)?;
        let roots_dir = view_dir.join("roots");
        create_dir(&roots_dir).map_err(map_infra_error)?;
        for root in &task.roots {
            let target = PathBuf::from(&root.path);
            let link = roots_dir.join(&root.id);
            if link.symlink_metadata().is_ok() {
                continue;
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&target, &link)
                    .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
            }
            #[cfg(not(unix))]
            {
                fs::write(&link, target.display().to_string())
                    .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
            }
        }
        fs::write(view_dir.join("TASK_ID"), task.id.as_str())
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        Ok(view_dir)
    }

    fn registry_tasks_dir(&self) -> PathBuf {
        self.state_root.join("registry").join("tasks")
    }

    fn scratch_dir(&self) -> PathBuf {
        self.state_root.join("scratch")
    }

    fn views_dir(&self) -> PathBuf {
        self.state_root.join("views")
    }
}

fn launch_adapter(adapter: &str, view_dir: &Path) -> Result<()> {
    match adapter {
        "opencode" => run_command("opencode", &[view_dir.display().to_string()]).map_err(|err| {
            anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to launch opencode: {err}"
            )))
        }),
        "shell" => {
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
            let status = Command::new(shell)
                .current_dir(view_dir)
                .status()
                .map_err(|err| {
                    anyhow!(TaskspaceError::ExternalCommand(format!(
                        "failed to launch shell: {err}"
                    )))
                })?;
            if !status.success() {
                return Err(anyhow!(TaskspaceError::ExternalCommand(format!(
                    "shell exited with status: {}",
                    status
                ))));
            }
            Ok(())
        }
        _ => Err(anyhow!(TaskspaceError::Usage(format!(
            "unsupported adapter: {}",
            adapter
        )))),
    }
}

fn parse_verify_command(command: &str) -> Result<(&str, Vec<&str>)> {
    let mut parts = command.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow!(TaskspaceError::Usage("verify command is empty".to_string())))?;
    if is_shell_program(program) {
        return Err(anyhow!(TaskspaceError::Usage(format!(
            "verify command cannot execute shell directly: {}",
            program
        ))));
    }
    let args = parts.collect::<Vec<_>>();
    Ok((program, args))
}

fn is_shell_program(program: &str) -> bool {
    matches!(
        program,
        "sh" | "bash" | "zsh" | "fish" | "cmd" | "powershell" | "pwsh"
    )
}

fn next_root_id(roots: &[Root]) -> String {
    let mut index = 1usize;
    loop {
        let candidate = format!("root_{index}");
        if roots.iter().all(|root| root.id != candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn ensure_layout(root: &Path) -> Result<()> {
    create_dir(root).map_err(map_infra_error)?;
    create_dir(&root.join("registry").join("tasks")).map_err(map_infra_error)?;
    create_dir(&root.join("scratch")).map_err(map_infra_error)?;
    create_dir(&root.join("cache")).map_err(map_infra_error)?;
    create_dir(&root.join("gc")).map_err(map_infra_error)?;
    create_dir(&root.join("views")).map_err(map_infra_error)?;
    Ok(())
}

fn new_task_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
    Ok(format!("tsk_{:x}", now.as_nanos()))
}

fn default_state_root() -> Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(home.join(".local").join("state").join("taskspace"))
}

fn slugify(title: &str) -> String {
    let mut slug = title
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug.trim_matches('-').to_string()
}

fn map_infra_error(err: anyhow::Error) -> anyhow::Error {
    anyhow!(TaskspaceError::Io(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn start_and_list_tasks_work() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let created = app
            .start_task(StartTaskRequest {
                title: "demo task".to_string(),
                entry_adapter: None,
            })
            .expect("start");
        assert_eq!(created.state, TaskState::Active);

        let list = app.list_tasks().expect("list");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "demo task");
    }

    #[test]
    fn attach_and_detach_root_work() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let project = temp.path().join("project");
        fs::create_dir_all(&project).expect("project");

        let task = app
            .start_task(StartTaskRequest {
                title: "attach task".to_string(),
                entry_adapter: None,
            })
            .expect("start");

        let attached = app
            .attach_root(AttachRootRequest {
                task_ref: task.id.as_str().to_string(),
                root_type: RootType::Dir,
                path: project,
                role: "source".to_string(),
                access: RootAccess::Rw,
                isolation: RootIsolation::Direct,
            })
            .expect("attach");

        app.detach_root(DetachRootRequest {
            task_ref: task.id.as_str().to_string(),
            root_id: attached.root_id,
        })
        .expect("detach");
    }

    #[test]
    fn finish_and_archive_work() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let task = app
            .start_task(StartTaskRequest {
                title: "finish task".to_string(),
                entry_adapter: None,
            })
            .expect("start");

        let state = app
            .finish_task(FinishTaskRequest {
                task_ref: task.id.as_str().to_string(),
                target_state: TaskState::Done,
            })
            .expect("finish");
        assert_eq!(state, TaskState::Done);

        app.archive_task(ArchiveTaskRequest {
            task_ref: task.id.as_str().to_string(),
        })
        .expect("archive");
    }

    #[test]
    fn next_root_id_skips_existing_suffixes() {
        let roots = vec![
            Root {
                id: "root_1".to_string(),
                root_type: RootType::Dir,
                path: "/tmp/a".to_string(),
                role: "source".to_string(),
                access: RootAccess::Rw,
                isolation: RootIsolation::Direct,
                branch: None,
                base_branch: None,
                include: Vec::new(),
                exclude: Vec::new(),
            },
            Root {
                id: "root_3".to_string(),
                root_type: RootType::Dir,
                path: "/tmp/b".to_string(),
                role: "docs".to_string(),
                access: RootAccess::Ro,
                isolation: RootIsolation::Direct,
                branch: None,
                base_branch: None,
                include: Vec::new(),
                exclude: Vec::new(),
            },
        ];

        assert_eq!(next_root_id(&roots), "root_2");
    }

    #[test]
    fn show_task_rejects_traversal_task_ref() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let err = app
            .show_task(ShowTaskRequest {
                task_ref: "../bad".to_string(),
            })
            .expect_err("must reject traversal");
        assert!(format!("{err}").contains("task id"));
    }

    #[test]
    fn parse_verify_command_rejects_empty() {
        let err = parse_verify_command("   ").expect_err("empty should fail");
        assert!(format!("{err}").contains("verify command is empty"));
    }

    #[test]
    fn parse_verify_command_rejects_shell_programs() {
        let err = parse_verify_command("sh -c ls").expect_err("shell should fail");
        assert!(format!("{err}").contains("cannot execute shell directly"));
    }

    #[test]
    fn attach_rejects_missing_path() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let task = app
            .start_task(StartTaskRequest {
                title: "missing root".to_string(),
                entry_adapter: None,
            })
            .expect("start");

        let err = app
            .attach_root(AttachRootRequest {
                task_ref: task.id.as_str().to_string(),
                root_type: RootType::Dir,
                path: temp.path().join("nope"),
                role: "source".to_string(),
                access: RootAccess::Rw,
                isolation: RootIsolation::Direct,
            })
            .expect_err("must fail");
        assert!(format!("{err}").contains("root path does not exist"));
    }

    #[test]
    fn detach_rejects_unknown_root_id() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let task = app
            .start_task(StartTaskRequest {
                title: "detach root".to_string(),
                entry_adapter: None,
            })
            .expect("start");

        let err = app
            .detach_root(DetachRootRequest {
                task_ref: task.id.as_str().to_string(),
                root_id: "root_missing".to_string(),
            })
            .expect_err("must fail");
        assert!(format!("{err}").contains("root id not found"));
    }

    #[test]
    fn finish_rejects_transition_from_archived() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let task = app
            .start_task(StartTaskRequest {
                title: "archive transition".to_string(),
                entry_adapter: None,
            })
            .expect("start");

        app.archive_task(ArchiveTaskRequest {
            task_ref: task.id.as_str().to_string(),
        })
        .expect("archive");

        let err = app
            .finish_task(FinishTaskRequest {
                task_ref: task.id.as_str().to_string(),
                target_state: TaskState::Active,
            })
            .expect_err("must fail");
        assert!(format!("{err}").contains("cannot transition task"));
    }

    #[test]
    fn verify_runs_commands_and_rejects_shell_wrapper() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let mut task = app
            .start_task(StartTaskRequest {
                title: "verify task".to_string(),
                entry_adapter: None,
            })
            .expect("start");

        task.verify.commands = vec!["true".to_string()];
        app.save_task(&task).expect("save");

        let out = app
            .verify_task(VerifyTaskRequest {
                task_ref: task.id.as_str().to_string(),
            })
            .expect("verify");
        assert_eq!(out.ran, vec!["true".to_string()]);

        task.verify.commands = vec!["sh -c true".to_string()];
        app.save_task(&task).expect("save");
        let err = app
            .verify_task(VerifyTaskRequest {
                task_ref: task.id.as_str().to_string(),
            })
            .expect_err("must reject shell");
        assert!(format!("{err}").contains("cannot execute shell directly"));
    }

    #[test]
    fn gc_removes_orphan_entries() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        ensure_layout(app.state_root()).expect("layout");
        let orphan_scratch = app.scratch_dir().join("tsk_orphan");
        let orphan_view = app.views_dir().join("tsk_orphan");
        fs::create_dir_all(&orphan_scratch).expect("scratch");
        fs::create_dir_all(&orphan_view).expect("view");

        let result = app.gc().expect("gc");
        assert_eq!(result.removed.len(), 2);
    }

    #[test]
    fn enter_rejects_unknown_adapter_without_launch() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");
        let task = app
            .start_task(StartTaskRequest {
                title: "enter task".to_string(),
                entry_adapter: Some("unknown".to_string()),
            })
            .expect("start");
        let err = app
            .enter_task(EnterTaskRequest {
                task_ref: task.id.as_str().to_string(),
                adapter: Some("unknown".to_string()),
            })
            .expect_err("must fail");
        assert!(format!("{err}").contains("unsupported adapter"));
    }
}
