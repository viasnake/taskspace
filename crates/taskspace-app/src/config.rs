//! Configuration file handling for taskspace.
//!
//! Loads and merges editor configurations from:
//! - `~/.config/taskspace/config.toml` (XDG Base Directory)
//! - Built-in defaults (opencode, codex, claude)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use taskspace_core::{EditorConfig, default_editors};

/// Global editor configuration loaded from config file and defaults.
#[derive(Debug, Clone)]
pub struct EditorRegistry {
    /// Editor name -> EditorConfig mapping (merged from config + defaults)
    editors: HashMap<String, EditorConfig>,
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

        // Load defaults
        for (name, config) in default_editors() {
            editors.insert(name.to_string(), config);
        }

        // Load config file (if exists) and override/add entries
        if let Some(path) = config_path
            && path.exists()
        {
            let file_config = load_config_file(path).context("failed to load config file")?;
            merge_config(&mut editors, file_config);
        }

        Ok(Self { editors })
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
        .or_else(|| dirs::home_dir().map(|h| h.join(".config")))?;

    Some(config_home.join("taskspace").join("config.toml"))
}

/// Loads the config file from disk.
fn load_config_file(path: &Path) -> Result<FileConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;

    toml::from_str(&content)
        .with_context(|| format!("failed to parse config file: {}", path.display()))
}

/// Merges file config into the editors map.
fn merge_config(editors: &mut HashMap<String, EditorConfig>, file_config: FileConfig) {
    for (name, editor_def) in file_config.editors {
        if let Some(command) = editor_def.command {
            editors.insert(name, EditorConfig { command });
        }
    }
}

/// Configuration file structure (TOML format).
#[derive(Debug, Clone, serde::Deserialize)]
struct FileConfig {
    #[serde(default)]
    editors: HashMap<String, EditorDefinition>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct EditorDefinition {
    command: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_from_none_returns_defaults() {
        let registry = EditorRegistry::load_from(None).expect("should load");

        assert!(registry.get("opencode").is_some());
        assert!(registry.get("codex").is_some());
        assert!(registry.get("claude").is_some());
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
"#,
        )
        .expect("write config");

        let registry = EditorRegistry::load_from(Some(&config_path)).expect("should load");

        // Custom editor should exist
        let config = registry.get("myeditor").expect("myeditor should exist");
        assert_eq!(config.command, vec!["myeditor", "{dir}"]);

        // Defaults should still exist
        assert!(registry.get("opencode").is_some());
    }

    #[test]
    fn test_editor_names_iter() {
        let registry = EditorRegistry::load_from(None).expect("should load");
        let names: Vec<&str> = registry.editor_names().collect();

        assert!(names.contains(&"opencode"));
        assert!(names.contains(&"codex"));
        assert!(names.contains(&"claude"));
    }
}
