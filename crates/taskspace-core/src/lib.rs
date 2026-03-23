use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TaskId(String);

impl TaskId {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        if !raw.starts_with("tsk_") {
            return Err(TaskspaceError::Usage(
                "task id must start with 'tsk_'".to_string(),
            ));
        }
        if raw.len() < 8 {
            return Err(TaskspaceError::Usage("task id is too short".to_string()));
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(TaskspaceError::Usage(
                "task id must contain only [A-Za-z0-9_-]".to_string(),
            ));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for TaskId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        TaskId::parse(&value).map_err(|err| err.to_string())
    }
}

impl From<TaskId> for String {
    fn from(value: TaskId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Active,
    Blocked,
    Review,
    Done,
    Archived,
}

impl TaskState {
    pub fn can_transition_to(self, next: Self) -> bool {
        match self {
            Self::Active => matches!(
                next,
                Self::Blocked | Self::Review | Self::Done | Self::Archived
            ),
            Self::Blocked => matches!(
                next,
                Self::Active | Self::Review | Self::Done | Self::Archived
            ),
            Self::Review => matches!(next, Self::Active | Self::Done | Self::Archived),
            Self::Done => matches!(next, Self::Review | Self::Archived),
            Self::Archived => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootType {
    Git,
    Dir,
    File,
    Artifact,
    Scratch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootAccess {
    Ro,
    Rw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootIsolation {
    Direct,
    Worktree,
    Copy,
    Symlink,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Root {
    pub id: String,
    #[serde(rename = "type")]
    pub root_type: RootType,
    pub path: String,
    pub role: String,
    pub access: RootAccess,
    pub isolation: RootIsolation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_branch: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub include: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub exclude: Vec<String>,
}

impl Root {
    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.id.trim().is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(TaskspaceError::Usage(
                "root id must contain only [A-Za-z0-9_-]".to_string(),
            ));
        }
        if self.path.trim().is_empty() {
            return Err(TaskspaceError::Usage(
                "root path must not be empty".to_string(),
            ));
        }
        if self.role.trim().is_empty() {
            return Err(TaskspaceError::Usage(
                "root role must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VerifySpec {
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_when: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TaskNotes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub slug: String,
    pub state: TaskState,
    pub updated_at: String,
    pub entry_adapter: String,
    #[serde(default)]
    pub roots: Vec<Root>,
    #[serde(default)]
    pub verify: VerifySpec,
    #[serde(default)]
    pub notes: TaskNotes,
}

impl Task {
    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.title.trim().is_empty() {
            return Err(TaskspaceError::Corrupt("task title is empty".to_string()));
        }
        if self.slug.trim().is_empty() {
            return Err(TaskspaceError::Corrupt("task slug is empty".to_string()));
        }
        if self.entry_adapter.trim().is_empty() {
            return Err(TaskspaceError::Corrupt(
                "task entry_adapter is empty".to_string(),
            ));
        }
        for root in &self.roots {
            root.validate()?;
        }
        Ok(())
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

    #[test]
    fn task_id_requires_tsk_prefix() {
        assert!(TaskId::parse("tsk_01abcde").is_ok());
        assert!(TaskId::parse("abc").is_err());
    }

    #[test]
    fn lifecycle_transition_rules_work() {
        assert!(TaskState::Active.can_transition_to(TaskState::Blocked));
        assert!(TaskState::Done.can_transition_to(TaskState::Archived));
        assert!(!TaskState::Archived.can_transition_to(TaskState::Active));
    }

    #[test]
    fn task_validation_checks_fields() {
        let task = Task {
            id: TaskId::parse("tsk_01abcde").expect("id"),
            title: "demo".to_string(),
            slug: "demo".to_string(),
            state: TaskState::Active,
            updated_at: "2026-03-24T00:00:00Z".to_string(),
            entry_adapter: "opencode".to_string(),
            roots: vec![Root {
                id: "root_a".to_string(),
                root_type: RootType::Dir,
                path: "/tmp".to_string(),
                role: "source".to_string(),
                access: RootAccess::Ro,
                isolation: RootIsolation::Direct,
                branch: None,
                base_branch: None,
                include: Vec::new(),
                exclude: Vec::new(),
            }],
            verify: VerifySpec::default(),
            notes: TaskNotes::default(),
        };

        task.validate().expect("valid task");
    }

    #[test]
    fn task_id_deserialize_rejects_invalid_value() {
        let parsed = serde_yaml::from_str::<TaskId>("'../bad'");
        assert!(parsed.is_err());
    }
}
