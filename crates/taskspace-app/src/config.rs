//! Configuration file handling for taskspace.
//!
//! Loads and merges editor configurations from:
//! - `~/.config/taskspace/config.toml` (XDG Base Directory)
//! - Built-in defaults from `taskspace_core::default_editors`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use taskspace_core::{EditorConfig, TaskspaceError, default_editors};

/// Global editor configuration loaded from config file and defaults.
#[derive(Debug, Clone)]
pub struct EditorRegistry {
    /// Editor name -> EditorConfig mapping (merged from config + defaults)
    editors: HashMap<String, EditorConfig>,
    /// Default editors used by `taskspace open` when `--editor` is omitted.
    default_open_editors: Vec<String>,
}

impl EditorRegistry {
    /// Creates a new editor registry by loading config and merging with defaults.
    ///
    /// Uses XDG Base Directory for config file location.
    pub fn load() -> Result<Self> {
        let config_path = config_file_path();
        Self::load_from(config_path.as_deref())
    }

    /// Creates a new editor registry from an optional config file path.
    ///
    /// If the path is None or the file doesn't exist, only defaults are used.
    pub fn load_from(config_path: Option<&Path>) -> Result<Self> {
        let mut editors: HashMap<String, EditorConfig> = HashMap::new();
        let mut default_open_editors = Vec::new();

        // Load defaults
        for (name, config) in default_editors() {
            editors.insert(name.to_string(), config);
            default_open_editors.push(name.to_string());
        }

        // Load config file (if exists) and override/add entries
        if let Some(path) = config_path
            && path.exists()
        {
            let file_config = load_config_file(path)?;
            merge_config(&mut editors, &file_config.editors)?;
            if let Some(configured_defaults) = &file_config.open.default_editors {
                default_open_editors =
                    validate_default_open_editors(configured_defaults, &editors)?;
            }
        }

        Ok(Self {
            editors,
            default_open_editors,
        })
    }

    /// Gets the configuration for a named editor.
    pub fn get(&self, name: &str) -> Option<&EditorConfig> {
        self.editors.get(name)
    }

    /// Gets all editor names.
    pub fn editor_names(&self) -> impl Iterator<Item = &str> {
        self.editors.keys().map(|s| s.as_str())
    }

    /// Gets all editor configurations with their names.
    pub fn all_editors(&self) -> impl Iterator<Item = (&str, &EditorConfig)> {
        self.editors.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Gets default editor names for implicit `open` requests.
    pub fn default_open_editors(&self) -> &[String] {
        &self.default_open_editors
    }
}

/// Returns the path to the config file, if it should exist.
///
/// Uses XDG Base Directory Specification:
/// - `$XDG_CONFIG_HOME/taskspace/config.toml` (if set)
/// - `$HOME/.config/taskspace/config.toml` (fallback)
pub fn config_file_path() -> Option<PathBuf> {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| home::home_dir().map(|h| h.join(".config")))?;

    Some(config_home.join("taskspace").join("config.toml"))
}

/// Loads the config file from disk.
fn load_config_file(path: &Path) -> Result<FileConfig> {
    let content = std::fs::read_to_string(path).map_err(|err| {
        anyhow!(TaskspaceError::Io(format!(
            "failed to read config file '{}': {err}",
            path.display()
        )))
    })?;

    toml::from_str(&content).map_err(|err| {
        anyhow!(TaskspaceError::Usage(format!(
            "failed to parse config file '{}': {err}",
            path.display()
        )))
    })
}

/// Merges file config into the editors map.
fn merge_config(
    editors: &mut HashMap<String, EditorConfig>,
    file_editors: &HashMap<String, EditorDefinition>,
) -> Result<()> {
    for (name, editor_def) in file_editors {
        if let Some(command) = editor_def.command.as_ref() {
            validate_editor_command(name, command)?;
            editors.insert(
                name.clone(),
                EditorConfig {
                    command: command.clone(),
                },
            );
        }
    }

    Ok(())
}

fn validate_default_open_editors(
    default_open_editors: &[String],
    editors: &HashMap<String, EditorConfig>,
) -> Result<Vec<String>> {
    if default_open_editors.is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(
            "invalid open config: default_editors cannot be empty".to_string()
        )));
    }

    let mut normalized = Vec::with_capacity(default_open_editors.len());
    for editor in default_open_editors {
        let name = editor.trim();
        if name.is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(
                "invalid open config: default_editors cannot contain empty names".to_string()
            )));
        }
        if !editors.contains_key(name) {
            return Err(anyhow!(TaskspaceError::Usage(format!(
                "invalid open config: unknown editor '{}' in default_editors",
                name
            ))));
        }
        normalized.push(name.to_string());
    }

    Ok(normalized)
}

fn validate_editor_command(editor_name: &str, command: &[String]) -> Result<()> {
    if command.is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(format!(
            "invalid editor config '{}': command cannot be empty",
            editor_name
        ))));
    }

    if command[0].trim().is_empty() {
        return Err(anyhow!(TaskspaceError::Usage(format!(
            "invalid editor config '{}': executable cannot be empty",
            editor_name
        ))));
    }

    Ok(())
}

/// Configuration file structure (TOML format).
#[derive(Debug, Clone, serde::Deserialize)]
struct FileConfig {
    #[serde(default)]
    editors: HashMap<String, EditorDefinition>,
    #[serde(default)]
    open: OpenConfig,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct EditorDefinition {
    command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct OpenConfig {
    default_editors: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_from_none_returns_defaults() {
        let registry = EditorRegistry::load_from(None).expect("should load");

        assert!(registry.get("vscode").is_some());
        assert!(registry.get("opencode").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("claude").is_some());
        assert_eq!(registry.default_open_editors()[0], "vscode");
    }

    #[test]
    fn test_load_from_nonexistent_file_returns_defaults() {
        let registry = EditorRegistry::load_from(Some(Path::new("/nonexistent/path.toml")))
            .expect("should load");

        assert!(registry.get("opencode").is_some());
    }

    #[test]
    fn test_load_from_config_file_overrides_defaults() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[editors.myeditor]
command = ["myeditor", "{dir}"]

[open]
default_editors = ["myeditor", "opencode"]
"#,
        )
        .expect("write config");

        let registry = EditorRegistry::load_from(Some(&config_path)).expect("should load");

        // Custom editor should exist
        let config = registry.get("myeditor").expect("myeditor should exist");
        assert_eq!(config.command, vec!["myeditor", "{dir}"]);

        // Defaults should still exist
        assert!(registry.get("opencode").is_some());
        assert_eq!(
            registry.default_open_editors(),
            &["myeditor".to_string(), "opencode".to_string()]
        );
    }

    #[test]
    fn test_editor_names_iter() {
        let registry = EditorRegistry::load_from(None).expect("should load");
        let names: Vec<&str> = registry.editor_names().collect();

        assert!(names.contains(&"vscode"));
        assert!(names.contains(&"opencode"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"claude"));
    }

    #[test]
    fn test_load_from_config_file_rejects_empty_command() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[editors.broken]
command = []
"#,
        )
        .expect("write config");

        let err =
            EditorRegistry::load_from(Some(&config_path)).expect_err("should reject empty command");
        assert!(format!("{err}").contains("command cannot be empty"));
    }

    #[test]
    fn test_load_from_config_file_rejects_empty_executable() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[editors.broken]
command = ["   ", "{dir}"]
"#,
        )
        .expect("write config");

        let err = EditorRegistry::load_from(Some(&config_path))
            .expect_err("should reject empty executable");
        assert!(format!("{err}").contains("executable cannot be empty"));
    }

    #[test]
    fn test_load_from_config_file_rejects_empty_default_editors() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[open]
default_editors = []
"#,
        )
        .expect("write config");

        let err = EditorRegistry::load_from(Some(&config_path))
            .expect_err("should reject empty default_editors");
        assert!(format!("{err}").contains("default_editors cannot be empty"));
    }

    #[test]
    fn test_load_from_config_file_rejects_unknown_default_editor() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("config.toml");

        std::fs::write(
            &config_path,
            r#"
[open]
default_editors = ["missing-editor"]
"#,
        )
        .expect("write config");

        let err = EditorRegistry::load_from(Some(&config_path))
            .expect_err("should reject unknown default editor");
        assert!(format!("{err}").contains("unknown editor 'missing-editor'"));
    }
}
