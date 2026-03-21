pub mod editor;

pub use editor::{EditorConfig, PlaceholderContext, default_editors, expand_placeholders};

use thiserror::Error;

pub const WORKSPACE_SCHEMA_VERSION: u32 = 3;

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
}
