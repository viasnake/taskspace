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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibleRepos {
    All,
    Selected(Vec<String>),
}

impl VisibleRepos {
    pub fn display_scope(&self) -> String {
        match self {
            Self::All => "all".to_string(),
            Self::Selected(repos) => repos.len().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub title: String,
    pub state: TaskState,
    pub updated_at: String,
    pub entry_adapter: String,
    pub visible_repos: VisibleRepos,
}

impl Task {
    pub fn validate(&self) -> Result<(), TaskspaceError> {
        if self.title.trim().is_empty() {
            return Err(TaskspaceError::Corrupt("task title is empty".to_string()));
        }
        if self.entry_adapter.trim().is_empty() {
            return Err(TaskspaceError::Corrupt(
                "task entry_adapter is empty".to_string(),
            ));
        }
        if let VisibleRepos::Selected(items) = &self.visible_repos
            && items.iter().any(|item| item.trim().is_empty())
        {
            return Err(TaskspaceError::Corrupt(
                "visible_repos has empty repository name".to_string(),
            ));
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

    fn sample_task(visible_repos: VisibleRepos) -> Task {
        Task {
            id: TaskId::parse("tsk_demo01").expect("task id"),
            title: "demo".to_string(),
            state: TaskState::Active,
            updated_at: "2026-03-30T00:00:00Z".to_string(),
            entry_adapter: "opencode".to_string(),
            visible_repos,
        }
    }

    #[test]
    fn task_id_parse_validates_format() {
        assert!(TaskId::parse("tsk_demo01").is_ok());
        assert!(TaskId::parse("demo01").is_err());
        assert!(TaskId::parse("tsk_a").is_err());
        assert!(TaskId::parse("tsk_demo!").is_err());
    }

    #[test]
    fn task_state_transitions_follow_lifecycle_rules() {
        assert!(TaskState::Active.can_transition_to(TaskState::Done));
        assert!(TaskState::Blocked.can_transition_to(TaskState::Review));
        assert!(TaskState::Review.can_transition_to(TaskState::Archived));
        assert!(TaskState::Done.can_transition_to(TaskState::Review));
        assert!(!TaskState::Done.can_transition_to(TaskState::Active));
        assert!(!TaskState::Archived.can_transition_to(TaskState::Active));
    }

    #[test]
    fn visible_repos_display_scope_matches_mode() {
        assert_eq!(VisibleRepos::All.display_scope(), "all");
        assert_eq!(
            VisibleRepos::Selected(vec!["app".to_string(), "infra".to_string()]).display_scope(),
            "2"
        );
    }

    #[test]
    fn task_validate_rejects_invalid_payloads() {
        let empty_title = Task {
            title: "   ".to_string(),
            ..sample_task(VisibleRepos::All)
        };
        assert!(empty_title.validate().is_err());

        let empty_adapter = Task {
            entry_adapter: "".to_string(),
            ..sample_task(VisibleRepos::All)
        };
        assert!(empty_adapter.validate().is_err());

        let empty_repo = sample_task(VisibleRepos::Selected(vec!["".to_string()]));
        assert!(empty_repo.validate().is_err());

        assert!(
            sample_task(VisibleRepos::Selected(vec!["app".to_string()]))
                .validate()
                .is_ok()
        );
    }
}
