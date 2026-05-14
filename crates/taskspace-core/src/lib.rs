use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SlotId(String);

impl SlotId {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        if raw.trim().is_empty() {
            return Err(TaskspaceError::Usage("slot id cannot be empty".to_string()));
        }
        if raw == "." || raw == ".." {
            return Err(TaskspaceError::Usage(
                "slot id cannot be '.' or '..'".to_string(),
            ));
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(TaskspaceError::Usage(
                "slot id must contain only [A-Za-z0-9_-]".to_string(),
            ));
        }
        Ok(Self(raw.to_string()))
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceSlot {
    pub id: SlotId,
    pub source: String,
    pub path: PathBuf,
    pub last_checkout: Option<String>,
    pub updated_at: String,
}

impl WorkspaceSlot {
    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.source.trim().is_empty() {
            return Err(TaskspaceError::Corrupt("slot source is empty".to_string()));
        }
        if self.path.as_os_str().is_empty() {
            return Err(TaskspaceError::Corrupt("slot path is empty".to_string()));
        }
        if let Some(value) = &self.last_checkout
            && value.trim().is_empty()
        {
            return Err(TaskspaceError::Corrupt(
                "slot last_checkout is empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceContext {
    pub schema_version: u32,
    pub slot: WorkspaceSlot,
}

impl WorkspaceContext {
    pub fn new(slot: WorkspaceSlot) -> Self {
        Self {
            schema_version: 1,
            slot,
        }
    }

    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.schema_version == 0 {
            return Err(TaskspaceError::Corrupt(
                "context schema_version must be positive".to_string(),
            ));
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_slot() -> WorkspaceSlot {
        WorkspaceSlot {
            id: SlotId::parse("slot-1").expect("slot id"),
            source: "/src/app".to_string(),
            path: PathBuf::from("/tmp/taskspace/workspaces/slot-1"),
            last_checkout: Some("main".to_string()),
            updated_at: "2026-05-14T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn slot_id_parse_validates_format() {
        assert!(SlotId::parse("slot-1").is_ok());
        assert!(SlotId::parse("agent_2").is_ok());
        assert!(SlotId::parse("").is_err());
        assert!(SlotId::parse("..").is_err());
        assert!(SlotId::parse("slot/1").is_err());
    }

    #[test]
    fn workspace_slot_validate_rejects_invalid_payloads() {
        let empty_source = WorkspaceSlot {
            source: "   ".to_string(),
            ..sample_slot()
        };
        assert!(empty_source.validate().is_err());

        let empty_path = WorkspaceSlot {
            path: PathBuf::new(),
            ..sample_slot()
        };
        assert!(empty_path.validate().is_err());

        let empty_checkout = WorkspaceSlot {
            last_checkout: Some("".to_string()),
            ..sample_slot()
        };
        assert!(empty_checkout.validate().is_err());

        assert!(sample_slot().validate().is_ok());
    }

    #[test]
    fn workspace_context_validate_rejects_invalid_payloads() {
        assert!(WorkspaceContext::new(sample_slot()).validate().is_ok());

        let invalid = WorkspaceContext {
            schema_version: 0,
            slot: sample_slot(),
        };
        assert!(invalid.validate().is_err());
    }
}
