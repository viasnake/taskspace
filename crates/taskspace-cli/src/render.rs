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
