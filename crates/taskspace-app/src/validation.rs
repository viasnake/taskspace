use std::path::Path;

use anyhow::{Result, anyhow};
use serde::Deserialize;
use taskspace_core::{TaskspaceError, WORKSPACE_SCHEMA_VERSION};
use taskspace_infra_fs::read_file;

use crate::map_infra_error;
use crate::spec;

pub fn validate_workspace_yaml(path: &Path) -> Result<()> {
    let content = read_file(path).map_err(map_infra_error)?;
    let parsed: WorkspaceModel = serde_yaml::from_str(&content)
        .map_err(|e| anyhow!(TaskspaceError::Corrupt(format!("invalid yaml: {e}"))))?;

    if parsed.version != WORKSPACE_SCHEMA_VERSION {
        return Err(anyhow!(TaskspaceError::Corrupt(format!(
            "unsupported workspace schema version: {}",
            parsed.version
        ))));
    }

    if parsed.name.is_empty() {
        return Err(anyhow!(TaskspaceError::Corrupt(
            "workspace name is empty".to_string()
        )));
    }

    Ok(())
}

pub fn validate_opencode_config(path: &Path) -> Result<()> {
    let content = read_file(path).map_err(map_infra_error)?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow!(TaskspaceError::Corrupt(format!("invalid json: {e}"))))?;

    let instructions = value
        .get("instructions")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            anyhow!(TaskspaceError::Corrupt(
                "instructions is missing".to_string()
            ))
        })?;

    let actual = instructions
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();

    if actual.len() != 6 {
        return Err(anyhow!(TaskspaceError::Corrupt(
            "instructions order does not match spec".to_string()
        )));
    }

    let expected = spec::default_instructions();
    if actual[0] != expected[0]
        || actual[1] != expected[1]
        || actual[3] != expected[3]
        || actual[4] != expected[4]
        || actual[5] != expected[5]
    {
        return Err(anyhow!(TaskspaceError::Corrupt(
            "instructions order does not match spec".to_string()
        )));
    }

    if !spec::ALLOWED_GLOBAL_SKILLS_PATHS.contains(&actual[2]) {
        return Err(anyhow!(TaskspaceError::Corrupt(
            "global SKILLS path must be ~/.taskspace/SKILLS.md or ~/.config/taskspace/SKILLS.md"
                .to_string(),
        )));
    }

    Ok(())
}

pub fn ensure_session_marker(session_dir: &Path) -> Result<()> {
    for rel in spec::SESSION_MARKERS {
        if !session_dir.join(rel).exists() {
            return Err(anyhow!(TaskspaceError::Corrupt(format!(
                "session marker missing: {}",
                rel
            ))));
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct WorkspaceModel {
    version: u32,
    name: String,
}
