use taskspace_core::TaskState;

use crate::execute::CommandResult;

pub fn render(result: CommandResult) -> Vec<String> {
    match result {
        CommandResult::Started(task) => vec![format!(
            "started task: {} ({})",
            task.id.as_str(),
            task.title
        )],
        CommandResult::Attached { root_id, warnings } => {
            let mut lines = vec![format!("attached root: {root_id}")];
            lines.extend(warnings.into_iter().map(|warn| format!("warning: {warn}")));
            lines
        }
        CommandResult::Detached => vec!["detached root".to_string()],
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
                        "{}\t{}\t{}\troots={}\t{}",
                        task.id,
                        task.title,
                        state_label(task.state),
                        task.roots_count,
                        task.updated_at
                    )
                })
                .collect()
        }
        CommandResult::TaskDetail(task) => {
            let mut lines = vec![
                format!("id: {}", task.id.as_str()),
                format!("title: {}", task.title),
                format!("slug: {}", task.slug),
                format!("state: {}", state_label(task.state)),
                format!("adapter: {}", task.entry_adapter),
                format!("updated_at: {}", task.updated_at),
                "roots:".to_string(),
            ];
            lines.extend(task.roots.into_iter().map(|root| {
                format!(
                    "- {} type={:?} role={} access={:?} isolation={:?} path={}",
                    root.id, root.root_type, root.role, root.access, root.isolation, root.path
                )
            }));
            if !task.verify.commands.is_empty() {
                lines.push("verify.commands:".to_string());
                lines.extend(
                    task.verify
                        .commands
                        .into_iter()
                        .map(|cmd| format!("- {cmd}")),
                );
            }
            lines
        }
        CommandResult::Verified { task_id, ran } => {
            let mut lines = vec![format!("verify passed: {}", task_id)];
            lines.extend(ran.into_iter().map(|cmd| format!("- {cmd}")));
            lines
        }
        CommandResult::Finished(state) => {
            vec![format!("task state updated: {}", state_label(state))]
        }
        CommandResult::Archived => vec!["task archived".to_string()],
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
    use taskspace_core::{Task, TaskId, TaskNotes, VerifySpec};

    #[test]
    fn render_list_empty_and_non_empty() {
        let empty = render(CommandResult::TaskList(Vec::new()));
        assert_eq!(empty, vec!["no tasks found".to_string()]);

        let lines = render(CommandResult::TaskList(vec![TaskSummary {
            id: "tsk_abc12345".to_string(),
            title: "demo".to_string(),
            state: TaskState::Active,
            roots_count: 2,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }]));
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("demo"));
    }

    #[test]
    fn render_enter_and_gc() {
        let entered = render(CommandResult::Entered {
            adapter: "opencode".to_string(),
            cwd: PathBuf::from("/tmp/view"),
            task_id: "tsk_abc12345".to_string(),
        });
        assert!(entered[0].contains("entered task"));

        let gc = render(CommandResult::Gc(GcResult {
            removed: vec![PathBuf::from("/tmp/a")],
        }));
        assert!(gc[0].contains("gc removed"));
        assert!(gc[1].contains("/tmp/a"));
    }

    #[test]
    fn render_task_detail_and_finished() {
        let task = Task {
            id: TaskId::parse("tsk_abc12345").expect("id"),
            title: "demo".to_string(),
            slug: "demo".to_string(),
            state: TaskState::Review,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            entry_adapter: "opencode".to_string(),
            roots: Vec::new(),
            verify: VerifySpec {
                commands: vec!["cargo test".to_string()],
                done_when: None,
            },
            notes: TaskNotes::default(),
        };
        let detail = render(CommandResult::TaskDetail(task));
        assert!(detail.iter().any(|line| line.contains("state: review")));

        let finished = render(CommandResult::Finished(TaskState::Done));
        assert_eq!(finished, vec!["task state updated: done".to_string()]);
    }
}
