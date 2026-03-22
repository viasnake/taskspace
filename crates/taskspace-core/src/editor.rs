//! Editor configuration and launching logic.
//!
//! This module handles editor-specific launch commands with support for
//! placeholder substitution ({dir}).

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Configuration for launching an editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorConfig {
    /// The command and arguments to launch the editor.
    /// Supports placeholders: {dir}
    pub command: Vec<String>,
}

/// Returns the default editor configurations built into taskspace.
///
/// These are used when no config file is present, or as fallbacks
/// when an editor is not defined in the config file.
pub fn default_editors() -> Vec<(&'static str, EditorConfig)> {
    vec![
        (
            "vscode",
            EditorConfig {
                command: vec!["code".to_string(), "{dir}".to_string()],
            },
        ),
        (
            "opencode",
            EditorConfig {
                command: vec!["opencode".to_string(), "{dir}".to_string()],
            },
        ),
        (
            "codex",
            EditorConfig {
                command: vec!["codex".to_string(), "{dir}".to_string()],
            },
        ),
        (
            "claude",
            EditorConfig {
                command: vec![
                    "claude".to_string(),
                    "--add-dir".to_string(),
                    "{dir}".to_string(),
                ],
            },
        ),
    ]
}

/// Context for placeholder expansion.
#[derive(Debug, Clone)]
pub struct PlaceholderContext {
    /// The session directory path.
    pub dir: String,
}

impl PlaceholderContext {
    /// Creates a new placeholder context from session directory.
    pub fn new(session_dir: &Path) -> Self {
        Self {
            dir: session_dir.display().to_string(),
        }
    }
}

/// Expands placeholders in editor command arguments.
///
/// Replaces `{dir}` with the session directory path.
///
/// # Arguments
///
/// * `command` - The command template with placeholders
/// * `context` - The placeholder values
///
/// # Returns
///
/// The expanded command with placeholders replaced
///
/// # Examples
///
/// ```
/// use taskspace_core::editor::{expand_placeholders, PlaceholderContext};
/// use std::path::Path;
///
/// let cmd = vec!["opencode".to_string(), "{dir}".to_string()];
/// let ctx = PlaceholderContext::new(Path::new("/sessions/test"));
/// let expanded = expand_placeholders(&cmd, &ctx);
/// assert_eq!(expanded[0], "opencode");
/// assert_eq!(expanded[1], "/sessions/test");
/// ```
pub fn expand_placeholders(command: &[String], context: &PlaceholderContext) -> Vec<String> {
    command
        .iter()
        .map(|arg| arg.replace("{dir}", &context.dir))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_dir_placeholder() {
        let cmd = vec!["opencode".to_string(), "{dir}".to_string()];
        let ctx = PlaceholderContext::new(Path::new("/sessions/my-session"));
        let expanded = expand_placeholders(&cmd, &ctx);

        assert_eq!(expanded[0], "opencode");
        assert_eq!(expanded[1], "/sessions/my-session");
    }

    #[test]
    fn test_expand_add_dir_command() {
        let cmd = vec![
            "claude".to_string(),
            "--add-dir".to_string(),
            "{dir}".to_string(),
        ];
        let ctx = PlaceholderContext::new(Path::new("/sessions/my-session"));
        let expanded = expand_placeholders(&cmd, &ctx);

        assert_eq!(expanded[0], "claude");
        assert_eq!(expanded[1], "--add-dir");
        assert_eq!(expanded[2], "/sessions/my-session");
    }

    #[test]
    fn test_default_editors_exist() {
        let editors = default_editors();
        let names: Vec<&str> = editors.iter().map(|(name, _)| *name).collect();
        assert!(names.contains(&"vscode"));
        assert!(names.contains(&"opencode"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"claude"));
    }

    #[test]
    fn test_editor_config_roundtrip() {
        let config = EditorConfig {
            command: vec!["opencode".to_string(), "{dir}".to_string()],
        };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: EditorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }
}
