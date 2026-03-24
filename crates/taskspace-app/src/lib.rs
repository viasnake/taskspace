use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, anyhow};
use chrono::Utc;
use taskspace_core::{Task, TaskId, TaskState, TaskspaceError, VisibleRepos};
use taskspace_infra_fs::{create_dir, list_directories, remove_dir_all, run_command};

const DEFAULT_ADAPTER: &str = "opencode";

#[derive(Debug, Clone)]
pub struct TaskspaceApp {
    workspace_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct StartTaskRequest {
    pub title: String,
    pub entry_adapter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UseReposRequest {
    pub task_ref: String,
    pub repos: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnterTaskRequest {
    pub task_ref: String,
}

#[derive(Debug, Clone)]
pub struct FinishTaskRequest {
    pub task_ref: String,
    pub target_state: TaskState,
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
    pub visible_scope: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct EnterTaskResult {
    pub adapter: String,
    pub cwd: PathBuf,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct GcResult {
    pub removed: Vec<PathBuf>,
}

impl TaskspaceApp {
    pub fn new(workspace_root: Option<PathBuf>) -> Result<Self> {
        Ok(Self {
            workspace_root: workspace_root.unwrap_or(default_workspace_root()?),
        })
    }

    pub fn start_task(&self, request: StartTaskRequest) -> Result<Task> {
        if request.title.trim().is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(
                "task title cannot be empty".to_string()
            )));
        }

        ensure_layout(&self.workspace_root)?;

        let id = new_task_id()?;
        let task_id = TaskId::parse(&id)?;
        let now = Utc::now().to_rfc3339();

        let scratch_path = self.scratch_dir().join(task_id.as_str());
        create_dir(&scratch_path).map_err(map_infra_error)?;

        let task = Task {
            id: task_id,
            title: request.title,
            state: TaskState::Active,
            updated_at: now,
            entry_adapter: request
                .entry_adapter
                .unwrap_or_else(|| DEFAULT_ADAPTER.to_string()),
            visible_repos: VisibleRepos::All,
        };
        task.validate()?;
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn list_repos(&self) -> Result<Vec<String>> {
        ensure_layout(&self.workspace_root)?;
        list_directories(&self.repos_dir()).map_err(map_infra_error)
    }

    pub fn use_repos(&self, request: UseReposRequest) -> Result<Task> {
        if request.repos.is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(
                "at least one repository must be specified".to_string(),
            )));
        }

        let available = self.list_repos()?;
        let available_set: HashSet<_> = available.iter().cloned().collect();

        let mut deduped = Vec::new();
        for name in request.repos {
            if !available_set.contains(&name) {
                return Err(anyhow!(TaskspaceError::NotFound(format!(
                    "repository '{}' does not exist under {}",
                    name,
                    self.repos_dir().display()
                ))));
            }
            if !deduped.contains(&name) {
                deduped.push(name);
            }
        }

        let mut task = self.load_task_from_ref(&request.task_ref)?;
        task.visible_repos = VisibleRepos::Selected(deduped);
        task.updated_at = Utc::now().to_rfc3339();
        self.save_task(&task)?;
        Ok(task)
    }

    pub fn list_tasks(&self) -> Result<Vec<TaskSummary>> {
        ensure_layout(&self.workspace_root)?;
        let mut out = Vec::new();
        for entry in list_directories(&self.tasks_dir()).map_err(map_infra_error)? {
            let task = self.load_task_by_id(entry.as_str())?;
            out.push(TaskSummary {
                id: task.id.as_str().to_string(),
                title: task.title,
                state: task.state,
                visible_scope: task.visible_repos.display_scope(),
                updated_at: task.updated_at,
            });
        }
        out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(out)
    }

    pub fn show_task(&self, request: ShowTaskRequest) -> Result<Task> {
        self.load_task_from_ref(request.task_ref.as_str())
    }

    pub fn enter_task(&self, request: EnterTaskRequest) -> Result<EnterTaskResult> {
        let task = self.load_task_from_ref(request.task_ref.as_str())?;
        let view_dir = self.prepare_view(&task)?;
        launch_adapter(&task.entry_adapter, &view_dir)?;

        Ok(EnterTaskResult {
            adapter: task.entry_adapter,
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

    pub fn gc(&self) -> Result<GcResult> {
        ensure_layout(&self.workspace_root)?;
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

    fn prepare_view(&self, task: &Task) -> Result<PathBuf> {
        let view_dir = self.views_dir().join(task.id.as_str());
        let repos_link_dir = view_dir.join("repos");
        let view_scratch_dir = view_dir.join("scratch");

        create_dir(&repos_link_dir).map_err(map_infra_error)?;
        create_dir(&view_scratch_dir).map_err(map_infra_error)?;

        for repo in self.resolve_visible_repos(task)? {
            let target = self.repos_dir().join(&repo);
            let link = repos_link_dir.join(&repo);
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

        self.write_taskspace_md(task, &view_dir)?;
        Ok(view_dir)
    }

    fn write_taskspace_md(&self, task: &Task, view_dir: &Path) -> Result<()> {
        let content = format!(
            "# TASKSPACE\n\nTask: {}\nTask ID: {}\nState: {}\n\nVisible repositories are under ./repos/.\nUse only the repositories relevant to this task.\nUse ./scratch/ for task-local temporary files.\n",
            task.title,
            task.id.as_str(),
            state_label(task.state)
        );
        fs::write(view_dir.join("TASKSPACE.md"), content)
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
    }

    fn resolve_visible_repos(&self, task: &Task) -> Result<Vec<String>> {
        let available = self.list_repos()?;
        match &task.visible_repos {
            VisibleRepos::All => Ok(available),
            VisibleRepos::Selected(selected) => {
                let available_set: HashSet<_> = available.iter().cloned().collect();
                for name in selected {
                    if !available_set.contains(name) {
                        return Err(anyhow!(TaskspaceError::NotFound(format!(
                            "repository '{}' configured in task but missing under {}",
                            name,
                            self.repos_dir().display(),
                        ))));
                    }
                }
                Ok(selected.clone())
            }
        }
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
        let task_yaml = self.tasks_dir().join(id).join("task.yaml");
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
        let task_dir = self.tasks_dir().join(task.id.as_str());
        create_dir(&task_dir).map_err(map_infra_error)?;
        let yaml = serde_yaml::to_string(task)
            .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
        let temp_path = task_dir.join("task.yaml.tmp");
        fs::write(&temp_path, yaml).map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))?;
        fs::rename(temp_path, task_dir.join("task.yaml"))
            .map_err(|err| anyhow!(TaskspaceError::Io(err.to_string())))
            .map(|_| ())
    }

    fn tasks_dir(&self) -> PathBuf {
        self.state_dir().join("tasks")
    }

    fn views_dir(&self) -> PathBuf {
        self.state_dir().join("views")
    }

    fn scratch_dir(&self) -> PathBuf {
        self.state_dir().join("scratch")
    }

    fn repos_dir(&self) -> PathBuf {
        self.workspace_root.join("repos")
    }

    fn state_dir(&self) -> PathBuf {
        self.workspace_root.join("state")
    }
}

fn ensure_layout(workspace_root: &Path) -> Result<()> {
    create_dir(workspace_root).map_err(map_infra_error)?;
    create_dir(&workspace_root.join("repos")).map_err(map_infra_error)?;
    create_dir(&workspace_root.join("state").join("tasks")).map_err(map_infra_error)?;
    create_dir(&workspace_root.join("state").join("views")).map_err(map_infra_error)?;
    create_dir(&workspace_root.join("state").join("scratch")).map_err(map_infra_error)?;
    Ok(())
}

fn default_workspace_root() -> Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(home.join("taskspace"))
}

fn new_task_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| anyhow!(TaskspaceError::Internal(err.to_string())))?;
    Ok(format!("tsk_{:x}", now.as_nanos()))
}

fn state_label(state: TaskState) -> &'static str {
    match state {
        TaskState::Active => "active",
        TaskState::Blocked => "blocked",
        TaskState::Review => "review",
        TaskState::Done => "done",
        TaskState::Archived => "archived",
    }
}

fn launch_adapter(adapter: &str, view_dir: &Path) -> Result<()> {
    match adapter {
        "opencode" => run_command("opencode", &[view_dir.display().to_string()]).map_err(|err| {
            anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to launch opencode: {err}"
            )))
        }),
        _ => Err(anyhow!(TaskspaceError::Usage(format!(
            "unsupported adapter: {}",
            adapter
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

    #[test]
    fn start_sets_visible_repos_all() {
        let temp = tempdir().expect("temp");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let task = app
            .start_task(StartTaskRequest {
                title: "demo task".to_string(),
                entry_adapter: Some("opencode".to_string()),
            })
            .expect("start");
        assert!(matches!(task.visible_repos, VisibleRepos::All));
    }

    #[test]
    fn use_repos_updates_task() {
        let temp = tempdir().expect("temp");
        fs::create_dir_all(temp.path().join("repos").join("app")).expect("mkdir");
        fs::create_dir_all(temp.path().join("repos").join("infra")).expect("mkdir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let task = app
            .start_task(StartTaskRequest {
                title: "demo task".to_string(),
                entry_adapter: None,
            })
            .expect("start");
        let updated = app
            .use_repos(UseReposRequest {
                task_ref: task.id.as_str().to_string(),
                repos: vec!["app".to_string(), "infra".to_string()],
            })
            .expect("use");

        assert!(matches!(
            updated.visible_repos,
            VisibleRepos::Selected(ref items) if items == &vec!["app".to_string(), "infra".to_string()]
        ));
    }

    #[test]
    fn list_shows_scope_count() {
        let temp = tempdir().expect("temp");
        fs::create_dir_all(temp.path().join("repos").join("app")).expect("mkdir");
        let app = TaskspaceApp::new(Some(temp.path().to_path_buf())).expect("app");

        let task = app
            .start_task(StartTaskRequest {
                title: "demo task".to_string(),
                entry_adapter: None,
            })
            .expect("start");
        app.use_repos(UseReposRequest {
            task_ref: task.id.as_str().to_string(),
            repos: vec!["app".to_string()],
        })
        .expect("use");

        let items = app.list_tasks().expect("list");
        assert_eq!(items[0].visible_scope, "1");
    }
}
