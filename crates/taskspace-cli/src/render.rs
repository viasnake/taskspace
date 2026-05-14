use std::path::Path;

use taskspace_app::{
    AddProjectResult, AddSlotsResult, EnterSlotResult, HookContextResult, InitWorkspaceResult,
    RemoveSlotResult, SyncSlotsResult,
};
use taskspace_core::{Project, WorkspaceSlot};

pub fn initialized(result: InitWorkspaceResult) -> Vec<String> {
    vec![format!("initialized taskspace at {}", result.root.display())]
}

pub fn project_added(result: AddProjectResult) -> Vec<String> {
    vec![format!(
        "registered project {}\tsource={}",
        result.project.id.as_str(),
        result.project.source
    )]
}

pub fn project_list(projects: Vec<Project>) -> Vec<String> {
    if projects.is_empty() {
        return vec!["no projects found".to_string()];
    }
    projects
        .into_iter()
        .map(|project| {
            format!(
                "{}\tsource={}\t{}",
                project.id.as_str(),
                project.source,
                project.updated_at
            )
        })
        .collect()
}

pub fn project_detail(project: Project) -> Vec<String> {
    vec![
        format!("id: {}", project.id.as_str()),
        format!("source: {}", project.source),
        format!("updated_at: {}", project.updated_at),
    ]
}

pub fn slots_added(result: AddSlotsResult) -> Vec<String> {
    result
        .slots
        .into_iter()
        .map(|slot| {
            format!(
                "{}\t{}",
                slot.slot_ref().as_string(),
                slot.path.display()
            )
        })
        .collect()
}

pub fn slot_list(slots: Vec<WorkspaceSlot>) -> Vec<String> {
    if slots.is_empty() {
        return vec!["no slots found".to_string()];
    }
    slots
        .into_iter()
        .map(|slot| {
            format!(
                "{}\t{}\tlast_sync={}\t{}",
                slot.slot_ref().as_string(),
                slot.path.display(),
                slot.last_sync_at.unwrap_or_else(|| "never".to_string()),
                slot.updated_at
            )
        })
        .collect()
}

pub fn slot_detail(slot: WorkspaceSlot) -> Vec<String> {
    vec![
        format!("ref: {}", slot.slot_ref().as_string()),
        format!("path: {}", slot.path.display()),
        format!(
            "last_sync_at: {}",
            slot.last_sync_at.unwrap_or_else(|| "never".to_string())
        ),
        format!("updated_at: {}", slot.updated_at),
    ]
}

pub fn slot_removed(result: RemoveSlotResult) -> Vec<String> {
    vec![format!(
        "removed {} from {}",
        result.slot.slot_ref().as_string(),
        result.slot.path.display()
    )]
}

pub fn sync_result(result: &SyncSlotsResult) -> Vec<String> {
    if result.statuses.is_empty() {
        return vec!["no slots found".to_string()];
    }
    result
        .statuses
        .iter()
        .map(|status| {
            let state = if status.success { "ok" } else { "failed" };
            format!(
                "{}\t{}\t{}",
                state,
                status.slot.slot_ref().as_string(),
                status.message
            )
        })
        .collect()
}

pub fn sync_error(result: &SyncSlotsResult) -> String {
    let failed = result.statuses.iter().filter(|status| !status.success).count();
    let mut lines = vec![format!("sync failed for {failed} slot(s)")];
    lines.extend(sync_result(result));
    lines.join("\n")
}

pub fn entered(result: &EnterSlotResult) -> Vec<String> {
    vec![format!(
        "entered {} with {} at {}",
        result.slot_ref.as_string(),
        result.agent,
        result.cwd.display()
    )]
}

pub fn hook_context(result: HookContextResult) -> Vec<String> {
    let mut lines = vec![format!("# {}", result.path.display())];
    lines.extend(result.content.lines().map(|line| line.to_string()));
    lines
}

#[allow(dead_code)]
pub fn path_only(path: &Path) -> Vec<String> {
    vec![path.display().to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use taskspace_core::{ProjectId, SlotId};

    fn sample_slot() -> WorkspaceSlot {
        WorkspaceSlot {
            project_id: ProjectId::parse("app").expect("project id"),
            id: SlotId::parse("agent-1").expect("slot id"),
            path: PathBuf::from("/tmp/taskspace/workspaces/app/agent-1"),
            last_sync_at: Some("2026-05-14T00:00:00Z".to_string()),
            updated_at: "2026-05-14T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn render_covers_primary_outputs() {
        assert_eq!(
            initialized(InitWorkspaceResult {
                root: PathBuf::from("/tmp/taskspace")
            }),
            vec!["initialized taskspace at /tmp/taskspace".to_string()]
        );

        assert_eq!(project_list(Vec::new()), vec!["no projects found".to_string()]);
        assert_eq!(slot_list(Vec::new()), vec!["no slots found".to_string()]);

        let detail = slot_detail(sample_slot());
        assert_eq!(detail[0], "ref: app:agent-1");
        assert_eq!(detail[2], "last_sync_at: 2026-05-14T00:00:00Z");
    }

    #[test]
    fn render_sync_and_context() {
        let sync = SyncSlotsResult {
            statuses: vec![taskspace_app::SyncSlotStatus {
                slot: sample_slot(),
                success: true,
                message: "fetched".to_string(),
            }],
        };
        assert_eq!(
            sync_result(&sync),
            vec!["ok\tapp:agent-1\tfetched".to_string()]
        );

        let context = hook_context(HookContextResult {
            path: PathBuf::from("/tmp/taskspace/workspaces/app/agent-1/.taskspace/context.yaml"),
            content: "schema_version: 2\n".to_string(),
        });
        assert_eq!(
            context,
            vec![
                "# /tmp/taskspace/workspaces/app/agent-1/.taskspace/context.yaml".to_string(),
                "schema_version: 2".to_string(),
            ]
        );
    }
}
