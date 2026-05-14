use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProjectId(String);

impl ProjectId {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        parse_identifier(raw, "project id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ProjectId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ProjectId::parse(&value).map_err(|err| err.to_string())
    }
}

impl From<ProjectId> for String {
    fn from(value: ProjectId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SlotId(String);

impl SlotId {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        parse_identifier(raw, "slot id").map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SlotId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        SlotId::parse(&value).map_err(|err| err.to_string())
    }
}

impl From<SlotId> for String {
    fn from(value: SlotId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SlotRef {
    project_id: ProjectId,
    slot_id: SlotId,
}

impl SlotRef {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        let Some((project, slot)) = raw.split_once(':') else {
            return Err(TaskspaceError::Usage(
                "slot reference must use <project>:<slot>".to_string(),
            ));
        };

        Ok(Self {
            project_id: ProjectId::parse(project)?,
            slot_id: SlotId::parse(slot)?,
        })
    }

    pub fn new(project_id: ProjectId, slot_id: SlotId) -> Self {
        Self {
            project_id,
            slot_id,
        }
    }

    pub fn project_id(&self) -> &ProjectId {
        &self.project_id
    }

    pub fn slot_id(&self) -> &SlotId {
        &self.slot_id
    }

    pub fn as_string(&self) -> String {
        format!("{}:{}", self.project_id.as_str(), self.slot_id.as_str())
    }
}

impl TryFrom<String> for SlotRef {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        SlotRef::parse(&value).map_err(|err| err.to_string())
    }
}

impl From<SlotRef> for String {
    fn from(value: SlotRef) -> Self {
        value.as_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub source: String,
    pub updated_at: String,
}

impl Project {
    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.source.trim().is_empty() {
            return Err(TaskspaceError::Corrupt(
                "project source is empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSlot {
    pub project_id: ProjectId,
    pub id: SlotId,
    pub path: PathBuf,
    pub last_sync_at: Option<String>,
    pub updated_at: String,
}

impl WorkspaceSlot {
    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.path.as_os_str().is_empty() {
            return Err(TaskspaceError::Corrupt("slot path is empty".to_string()));
        }
        if let Some(value) = &self.last_sync_at
            && value.trim().is_empty()
        {
            return Err(TaskspaceError::Corrupt(
                "slot last_sync_at is empty".to_string(),
            ));
        }
        Ok(())
    }

    pub fn slot_ref(&self) -> SlotRef {
        SlotRef::new(self.project_id.clone(), self.id.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub schema_version: u32,
    pub project: Project,
    pub slot: WorkspaceSlot,
}

impl WorkspaceContext {
    pub fn new(project: Project, slot: WorkspaceSlot) -> Self {
        Self {
            schema_version: 2,
            project,
            slot,
        }
    }

    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.schema_version != 2 {
            return Err(TaskspaceError::Corrupt(
                "context schema_version must be 2".to_string(),
            ));
        }
        self.project.validate()?;
        self.slot.validate()
    }
}

#[derive(Debug, Error)]
pub enum TaskspaceError {
    #[error("error: {0}")]
    Usage(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("io error: {0}")]
    Io(String),

    #[error("corrupt state: {0}")]
    Corrupt(String),

    #[error("external command failed: {0}")]
    ExternalCommand(String),

    #[error("internal error: {0}")]
    Internal(String),
}

fn parse_identifier(raw: &str, label: &str) -> Result<String, TaskspaceError> {
    if raw.trim().is_empty() {
        return Err(TaskspaceError::Usage(format!("{label} cannot be empty")));
    }
    if raw == "." || raw == ".." {
        return Err(TaskspaceError::Usage(format!(
            "{label} cannot be '.' or '..'"
        )));
    }
    if !raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(TaskspaceError::Usage(format!(
            "{label} must contain only [A-Za-z0-9_-]"
        )));
    }
    Ok(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_project() -> Project {
        Project {
            id: ProjectId::parse("app").expect("project id"),
            source: "/src/app".to_string(),
            updated_at: "2026-05-14T00:00:00Z".to_string(),
        }
    }

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
    fn identifiers_validate_format() {
        assert!(ProjectId::parse("app").is_ok());
        assert!(SlotId::parse("agent_2").is_ok());
        assert!(ProjectId::parse("").is_err());
        assert!(SlotId::parse("..").is_err());
        assert!(ProjectId::parse("slot/1").is_err());
    }

    #[test]
    fn slot_ref_requires_project_and_slot() {
        let slot_ref = SlotRef::parse("app:agent-1").expect("slot ref");
        assert_eq!(slot_ref.project_id().as_str(), "app");
        assert_eq!(slot_ref.slot_id().as_str(), "agent-1");
        assert!(SlotRef::parse("agent-1").is_err());
    }

    #[test]
    fn project_and_slot_validation_reject_invalid_payloads() {
        let empty_source = Project {
            source: "   ".to_string(),
            ..sample_project()
        };
        assert!(empty_source.validate().is_err());

        let empty_path = WorkspaceSlot {
            path: PathBuf::new(),
            ..sample_slot()
        };
        assert!(empty_path.validate().is_err());

        let empty_sync = WorkspaceSlot {
            last_sync_at: Some("".to_string()),
            ..sample_slot()
        };
        assert!(empty_sync.validate().is_err());

        assert!(sample_project().validate().is_ok());
        assert!(sample_slot().validate().is_ok());
    }

    #[test]
    fn workspace_context_validate_rejects_invalid_payloads() {
        assert!(WorkspaceContext::new(sample_project(), sample_slot())
            .validate()
            .is_ok());

        let invalid = WorkspaceContext {
            schema_version: 1,
            project: sample_project(),
            slot: sample_slot(),
        };
        assert!(invalid.validate().is_err());
    }
}
