use std::path::Path;

use anyhow::{Result, anyhow};
use taskspace_core::{TaskspaceError, WORKSPACE_SCHEMA_VERSION};
use taskspace_infra_fs::read_file;

use crate::map_infra_error;
use crate::spec;
use crate::template::{WorkspaceModel, manifest_validation_errors};

pub fn validate_workspace_yaml(path: &Path, expected_session_name: &str) -> Result<WorkspaceModel> {
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

    if parsed.name != expected_session_name {
        return Err(anyhow!(TaskspaceError::Corrupt(format!(
            "workspace name '{}' does not match session directory '{}'",
            parsed.name, expected_session_name
        ))));
    }

    if parsed.layout_version == 0 {
        return Err(anyhow!(TaskspaceError::Corrupt(
            "layout_version must be >= 1".to_string()
        )));
    }

    if let Some(manifest) = &parsed.manifest {
        let manifest_errors = manifest_validation_errors(manifest);
        if !manifest_errors.is_empty() {
            return Err(anyhow!(TaskspaceError::Corrupt(format!(
                "invalid manifest: {}",
                manifest_errors.join("; ")
            ))));
        }
    }

    if let Some(template) = &parsed.template {
        if template.ref_path.trim().is_empty() {
            return Err(anyhow!(TaskspaceError::Corrupt(
                "template.ref must not be empty".to_string()
            )));
        }
        if !template.digest.starts_with("sha256:") || template.digest.len() <= 7 {
            return Err(anyhow!(TaskspaceError::Corrupt(
                "template.digest must be in format 'sha256:<hex>'".to_string()
            )));
        }
    }

    Ok(parsed)
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
