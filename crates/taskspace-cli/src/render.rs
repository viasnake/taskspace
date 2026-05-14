use std::path::Path;

use taskspace_app::{CheckoutResult, HookContextResult, InitWorkspacesResult};
use taskspace_core::WorkspaceSlot;

pub fn initialized(result: InitWorkspacesResult) -> Vec<String> {
    result
        .slots
        .into_iter()
        .map(|slot| format!("{}\t{}", slot.id.as_str(), slot.path.display()))
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
                "{}\t{}\tcheckout={}\t{}",
                slot.id.as_str(),
                slot.path.display(),
                slot.last_checkout.unwrap_or_else(|| "unknown".to_string()),
                slot.updated_at
            )
        })
        .collect()
}

pub fn slot_detail(slot: WorkspaceSlot) -> Vec<String> {
    vec![
        format!("id: {}", slot.id.as_str()),
        format!("source: {}", slot.source),
        format!("path: {}", slot.path.display()),
        format!(
            "checkout: {}",
            slot.last_checkout.unwrap_or_else(|| "unknown".to_string())
        ),
        format!("updated_at: {}", slot.updated_at),
    ]
}

pub fn checked_out(result: CheckoutResult) -> Vec<String> {
    vec![format!(
        "{} checked out {} at {}",
        result.slot.id.as_str(),
        result.git_ref,
        result.slot.path.display()
    )]
}

pub fn entered(agent: &str, cwd: &Path, slot_id: &str) -> Vec<String> {
    vec![format!(
        "entered {} with {} at {}",
        slot_id,
        agent,
        cwd.display()
    )]
}

pub fn hook_context(result: HookContextResult) -> Vec<String> {
    let mut lines = vec![format!("# {}", result.path.display())];
    lines.extend(result.content.lines().map(|line| line.to_string()));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use taskspace_app::{CheckoutResult, HookContextResult, InitWorkspacesResult};
    use taskspace_core::{SlotId, WorkspaceSlot};

    fn sample_slot() -> WorkspaceSlot {
        WorkspaceSlot {
            id: SlotId::parse("agent-1").expect("slot id"),
            source: "/src/app".to_string(),
            path: PathBuf::from("/tmp/taskspace/workspaces/agent-1"),
            last_checkout: Some("main".to_string()),
            updated_at: "2026-05-14T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn render_covers_primary_outputs() {
        assert_eq!(
            initialized(InitWorkspacesResult {
                slots: vec![sample_slot()]
            }),
            vec!["agent-1\t/tmp/taskspace/workspaces/agent-1".to_string()]
        );
        assert_eq!(slot_list(Vec::new()), vec!["no slots found".to_string()]);
        assert_eq!(
            slot_list(vec![WorkspaceSlot {
                id: SlotId::parse("agent-1").expect("slot id"),
                source: "/src/app".to_string(),
                path: PathBuf::from("/tmp/taskspace/workspaces/agent-1"),
                last_checkout: Some("main".to_string()),
                updated_at: "2026-05-14T00:00:00Z".to_string(),
            }]),
            vec![
                "agent-1\t/tmp/taskspace/workspaces/agent-1\tcheckout=main\t2026-05-14T00:00:00Z"
                    .to_string()
            ]
        );

        let detail = slot_detail(sample_slot());
        assert_eq!(detail[0], "id: agent-1");
        assert_eq!(detail[3], "checkout: main");

        assert_eq!(
            checked_out(CheckoutResult {
                slot: sample_slot(),
                git_ref: "feature/a".to_string(),
            }),
            vec!["agent-1 checked out feature/a at /tmp/taskspace/workspaces/agent-1".to_string()]
        );
    }

    #[test]
    fn render_formats_enter_and_hook_context() {
        assert_eq!(
            entered(
                "codex",
                &PathBuf::from("/tmp/taskspace/workspaces/agent-1"),
                "agent-1"
            ),
            vec!["entered agent-1 with codex at /tmp/taskspace/workspaces/agent-1".to_string()]
        );

        let context = hook_context(HookContextResult {
            path: PathBuf::from("/tmp/taskspace/workspaces/agent-1/.taskspace/context.yaml"),
            content: "schema_version: 1\n".to_string(),
        });
        assert_eq!(
            context,
            vec![
                "# /tmp/taskspace/workspaces/agent-1/.taskspace/context.yaml".to_string(),
                "schema_version: 1".to_string(),
            ]
        );
    }
}
