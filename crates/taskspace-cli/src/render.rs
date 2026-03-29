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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use taskspace_app::{GcResult, TaskSummary};
    use taskspace_core::{Task, TaskId};

    fn sample_task(visible_repos: VisibleRepos) -> Task {
        Task {
            id: TaskId::parse("tsk_demo01").expect("task id"),
            title: "demo".to_string(),
            state: TaskState::Review,
            updated_at: "2026-03-30T00:00:00Z".to_string(),
            entry_adapter: "opencode".to_string(),
            visible_repos,
        }
    }

    #[test]
    fn render_covers_primary_command_results() {
        assert_eq!(
            render(CommandResult::Started(sample_task(VisibleRepos::All))),
            vec!["tsk_demo01".to_string()]
        );
        assert_eq!(
            render(CommandResult::Repos(Vec::new())),
            vec!["no repositories found".to_string()]
        );
        assert_eq!(
            render(CommandResult::Repos(vec!["app".to_string()])),
            vec!["app".to_string()]
        );
        assert_eq!(
            render(CommandResult::Scoped(sample_task(VisibleRepos::All))),
            vec!["visible_repos: all".to_string()]
        );
        assert_eq!(
            render(CommandResult::Scoped(sample_task(VisibleRepos::Selected(
                vec!["app".to_string(), "infra".to_string(),]
            )))),
            vec!["visible_repos: [app, infra]".to_string()]
        );
        assert_eq!(
            render(CommandResult::Finished(TaskState::Archived)),
            vec!["task state updated: archived".to_string()]
        );
    }

    #[test]
    fn render_formats_detail_entry_list_and_gc_results() {
        let entered = render(CommandResult::Entered {
            adapter: "opencode".to_string(),
            cwd: PathBuf::from("/tmp/taskspace"),
            task_id: "tsk_demo01".to_string(),
        });
        assert_eq!(
            entered,
            vec!["entered task tsk_demo01 with adapter opencode at /tmp/taskspace".to_string()]
        );

        let listed = render(CommandResult::TaskList(vec![TaskSummary {
            id: "tsk_demo01".to_string(),
            title: "demo".to_string(),
            state: TaskState::Active,
            visible_scope: "all".to_string(),
            updated_at: "2026-03-30T00:00:00Z".to_string(),
        }]));
        assert_eq!(
            listed,
            vec!["tsk_demo01\tdemo\tactive\tvisible=all\t2026-03-30T00:00:00Z".to_string()]
        );
        assert_eq!(
            render(CommandResult::TaskList(Vec::new())),
            vec!["no tasks found".to_string()]
        );

        let detailed = render(CommandResult::TaskDetail(sample_task(
            VisibleRepos::Selected(vec!["app".to_string()]),
        )));
        assert_eq!(detailed[0], "id: tsk_demo01");
        assert_eq!(detailed[4], "visible_repos: app");

        assert_eq!(
            render(CommandResult::Gc(GcResult {
                removed: Vec::new()
            })),
            vec!["gc: nothing to remove".to_string()]
        );
        let gc = render(CommandResult::Gc(GcResult {
            removed: vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")],
        }));
        assert_eq!(gc[0], "gc removed 2 entries");
        assert_eq!(gc[1], "- /tmp/a");
        assert_eq!(gc[2], "- /tmp/b");
    }
}
