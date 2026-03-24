use taskspace_core::{TaskState, VisibleRepos};

use crate::execute::CommandResult;

pub fn render(result: CommandResult) -> Vec<String> {
    match result {
        CommandResult::Started(task) => vec![task.id.as_str().to_string()],
        CommandResult::Repos(repos) => {
            if repos.is_empty() {
                return vec!["no repositories found".to_string()];
            }
            repos
        }
        CommandResult::Scoped(task) => match task.visible_repos {
            VisibleRepos::All => vec!["visible_repos: all".to_string()],
            VisibleRepos::Selected(repos) => vec![format!("visible_repos: [{}]", repos.join(", "))],
        },
        CommandResult::Entered {
            adapter,
            cwd,
            task_id,
        } => vec![format!(
            "entered task {} with adapter {} at {}",
            task_id,
            adapter,
            cwd.display()
        )],
        CommandResult::TaskList(tasks) => {
            if tasks.is_empty() {
                return vec!["no tasks found".to_string()];
            }
            tasks
                .into_iter()
                .map(|task| {
                    format!(
                        "{}\t{}\t{}\tvisible={}\t{}",
                        task.id,
                        task.title,
                        state_label(task.state),
                        task.visible_scope,
                        task.updated_at
                    )
                })
                .collect()
        }
        CommandResult::TaskDetail(task) => {
            let visible = match task.visible_repos {
                VisibleRepos::All => "all".to_string(),
                VisibleRepos::Selected(repos) => repos.join(", "),
            };
            vec![
                format!("id: {}", task.id.as_str()),
                format!("title: {}", task.title),
                format!("state: {}", state_label(task.state)),
                format!("adapter: {}", task.entry_adapter),
                format!("visible_repos: {}", visible),
                format!("updated_at: {}", task.updated_at),
            ]
        }
        CommandResult::Finished(state) => {
            vec![format!("task state updated: {}", state_label(state))]
        }
        CommandResult::Gc(result) => {
            if result.removed.is_empty() {
                return vec!["gc: nothing to remove".to_string()];
            }
            let mut lines = vec![format!("gc removed {} entries", result.removed.len())];
            lines.extend(
                result
                    .removed
                    .into_iter()
                    .map(|path| format!("- {}", path.display())),
            );
            lines
        }
    }
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
