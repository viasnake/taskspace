use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const WORKSPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EditorKind {
    #[default]
    Opencode,
    Code,
}

impl fmt::Display for EditorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opencode => write!(f, "opencode"),
            Self::Code => write!(f, "code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionName(String);

impl SessionName {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        if raw.is_empty() {
            return Err(TaskspaceError::Usage(
                "session name cannot be empty".to_string(),
            ));
        }

        if raw == "." || raw == ".." {
            return Err(TaskspaceError::Usage(
                "session name cannot be '.' or '..'".to_string(),
            ));
        }

        let allowed = raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
        if !allowed {
            return Err(TaskspaceError::Usage(
                "session name must contain only [A-Za-z0-9._-]".to_string(),
            ));
        }

        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSpec {
    pub name: String,
    pub source: String,
}

impl RepoSpec {
    pub fn parse(raw: &str) -> Result<Self, TaskspaceError> {
        let (name, source) = raw.split_once('=').ok_or_else(|| {
            TaskspaceError::Usage("repo must be in <name>=<path|url> format".to_string())
        })?;

        if name.is_empty() || source.is_empty() {
            return Err(TaskspaceError::Usage(
                "repo name and source must be non-empty".to_string(),
            ));
        }

        let valid_name = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !valid_name {
            return Err(TaskspaceError::Usage(
                "repo name must contain only [A-Za-z0-9_-]".to_string(),
            ));
        }

        Ok(Self {
            name: name.to_string(),
            source: source.to_string(),
        })
    }
}

#[derive(Debug, Error)]
pub enum TaskspaceError {
    #[error("usage error: {0}")]
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
    fn session_name_accepts_simple_name() {
        let name = SessionName::parse("feature-123").expect("valid name");
        assert_eq!(name.as_str(), "feature-123");
    }

    #[test]
    fn session_name_rejects_invalid_chars() {
        let err = SessionName::parse("feature/123").expect_err("invalid name");
        assert!(matches!(err, TaskspaceError::Usage(_)));
    }

    #[test]
    fn repo_spec_parses_name_and_source() {
        let spec = RepoSpec::parse("app=https://example.com/app.git").expect("valid repo spec");
        assert_eq!(spec.name, "app");
        assert_eq!(spec.source, "https://example.com/app.git");
    }

    #[test]
    fn editor_kind_display_and_default() {
        assert_eq!(EditorKind::default().to_string(), "opencode");
        assert_eq!(EditorKind::Code.to_string(), "code");
    }

    #[test]
    fn session_name_rejects_empty_and_dot_variants() {
        assert!(matches!(
            SessionName::parse(""),
            Err(TaskspaceError::Usage(_))
        ));
        assert!(matches!(
            SessionName::parse("."),
            Err(TaskspaceError::Usage(_))
        ));
        assert!(matches!(
            SessionName::parse(".."),
            Err(TaskspaceError::Usage(_))
        ));
    }

    #[test]
    fn repo_spec_rejects_invalid_inputs() {
        assert!(matches!(
            RepoSpec::parse("missing"),
            Err(TaskspaceError::Usage(_))
        ));
        assert!(matches!(
            RepoSpec::parse("a="),
            Err(TaskspaceError::Usage(_))
        ));
        assert!(matches!(
            RepoSpec::parse("=b"),
            Err(TaskspaceError::Usage(_))
        ));
        assert!(matches!(
            RepoSpec::parse("bad/name=src"),
            Err(TaskspaceError::Usage(_))
        ));
    }
}
