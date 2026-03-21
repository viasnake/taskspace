use std::collections::HashSet;
use std::path::{Component, Path};

use anyhow::{Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use taskspace_core::{TaskspaceError, WORKSPACE_SCHEMA_VERSION};
use taskspace_infra_fs::{canonicalize_if_exists, read_file, write_file};

use crate::map_infra_error;
use crate::spec;

const TEMPLATE_SCHEMA_VERSION: u32 = 1;

pub fn create_base_structure(session_dir: &Path) -> Result<()> {
    let dirs = [
        "context",
        "repos",
        "references",
        "notes",
        "output",
        ".opencode",
        ".opencode/agents",
        ".opencode/plugins",
    ];

    for d in dirs {
        taskspace_infra_fs::create_dir(&session_dir.join(d)).map_err(map_infra_error)?;
    }

    Ok(())
}

pub fn resolve_workspace_model(
    session_name: &str,
    template_path: Option<&Path>,
) -> Result<WorkspaceModel> {
    match template_path {
        Some(path) => resolve_workspace_model_from_template(session_name, path),
        None => Ok(WorkspaceModel {
            version: WORKSPACE_SCHEMA_VERSION,
            name: session_name.to_string(),
            created_at: Utc::now().to_rfc3339(),
            layout_version: 1,
            created_by: CreatedBy::Manual,
            template: None,
            manifest: None,
        }),
    }
}

pub fn write_templates(session_dir: &Path, workspace: &WorkspaceModel) -> Result<()> {
    let session_md = format!(
        "# Session: {}\n\n## Objective\nDescribe the task objective.\n\n## Scope\nDescribe the scope and boundaries.\n",
        workspace.name
    );
    write_relative_file(session_dir, "SESSION.md", &session_md)?;

    let agents_md = "# Agents Rules\n\n- Do not commit context files.\n- Avoid destructive commands unless explicitly requested.\n";
    write_relative_file(session_dir, "AGENTS.md", agents_md)?;

    write_relative_file(session_dir, "context/MEMORY.md", "# MEMORY\n")?;
    write_relative_file(session_dir, "context/PLAN.md", "# PLAN\n")?;
    write_relative_file(session_dir, "context/CONSTRAINTS.md", "# CONSTRAINTS\n")?;
    write_relative_file(session_dir, "context/DECISIONS.md", "# DECISIONS\n")?;
    write_relative_file(session_dir, "context/LINKS.md", "# LINKS\n")?;

    let opencode_json = serde_json::json!({
        "instructions": spec::default_instructions(),
        "permission": {
            "edit": "ask",
            "bash": "ask",
        }
    });
    write_relative_file(
        session_dir,
        ".opencode/opencode.jsonc",
        &serde_json::to_string_pretty(&opencode_json)?,
    )?;

    write_relative_file(
        session_dir,
        "workspace.yaml",
        &serde_yaml::to_string(workspace)?,
    )?;

    Ok(())
}

fn write_relative_file(base: &Path, rel: &str, content: &str) -> Result<()> {
    write_file(&base.join(rel), content).map_err(map_infra_error)
}

fn resolve_workspace_model_from_template(
    session_name: &str,
    path: &Path,
) -> Result<WorkspaceModel> {
    let raw = read_file(path).map_err(map_infra_error)?;
    let parsed: TemplateModel = serde_yaml::from_str(&raw).map_err(|err| {
        anyhow!(TaskspaceError::Usage(format!(
            "invalid template yaml: {err}"
        )))
    })?;

    if parsed.version != TEMPLATE_SCHEMA_VERSION {
        return Err(anyhow!(TaskspaceError::Usage(format!(
            "unsupported template schema version: {}",
            parsed.version
        ))));
    }

    if let Some(manifest) = &parsed.manifest {
        let manifest_errors = manifest_validation_errors(manifest);
        if !manifest_errors.is_empty() {
            return Err(anyhow!(TaskspaceError::Usage(format!(
                "invalid template manifest: {}",
                manifest_errors.join("; ")
            ))));
        }
    }

    let layout_version = parsed
        .defaults
        .as_ref()
        .and_then(|defaults| defaults.layout_version)
        .unwrap_or(1);
    if layout_version == 0 {
        return Err(anyhow!(TaskspaceError::Usage(
            "template defaults.layout_version must be >= 1".to_string()
        )));
    }

    let digest = format!("sha256:{:x}", Sha256::digest(raw.as_bytes()));
    let template_ref_path = canonicalize_if_exists(&path.display().to_string())
        .unwrap_or_else(|| path.to_path_buf())
        .display()
        .to_string();

    Ok(WorkspaceModel {
        version: WORKSPACE_SCHEMA_VERSION,
        name: session_name.to_string(),
        created_at: Utc::now().to_rfc3339(),
        layout_version,
        created_by: CreatedBy::Template,
        template: Some(TemplateRef {
            ref_path: template_ref_path,
            digest,
        }),
        manifest: parsed.manifest,
    })
}

pub fn manifest_validation_errors(manifest: &Manifest) -> Vec<String> {
    let mut errors = Vec::new();
    if manifest.projects.is_empty() {
        errors.push("manifest.projects must not be empty".to_string());
        return errors;
    }

    let mut ids = HashSet::new();
    let mut targets = HashSet::new();

    for project in &manifest.projects {
        if project.id.is_empty() {
            errors.push("manifest project id must not be empty".to_string());
        }
        if !project
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            errors.push(format!(
                "manifest project '{}' has invalid id (allowed: [A-Za-z0-9._-])",
                project.id
            ));
        }
        if !ids.insert(project.id.clone()) {
            errors.push(format!("duplicate manifest project id: {}", project.id));
        }

        if project.source.trim().is_empty() {
            errors.push(format!(
                "manifest project '{}' source must not be empty",
                project.id
            ));
        }
        if project.source.starts_with('-') {
            errors.push(format!(
                "manifest project '{}' source cannot start with '-': {}",
                project.id, project.source
            ));
        }
        if !is_allowed_source(&project.source) {
            errors.push(format!(
                "manifest project '{}' has unsupported source: {}",
                project.id, project.source
            ));
        }

        if let Some(revision) = &project.revision
            && revision.trim().is_empty()
        {
            errors.push(format!(
                "manifest project '{}' revision must not be empty",
                project.id
            ));
        }

        if project.target.is_empty() {
            errors.push(format!(
                "manifest project '{}' target must not be empty",
                project.id
            ));
            continue;
        }
        if !targets.insert(project.target.clone()) {
            errors.push(format!("duplicate manifest target: {}", project.target));
        }

        let path = Path::new(&project.target);
        if path.is_absolute() {
            errors.push(format!(
                "manifest project '{}' target must be relative: {}",
                project.id, project.target
            ));
        }
        if path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
        {
            errors.push(format!(
                "manifest project '{}' target cannot contain '..': {}",
                project.id, project.target
            ));
        }
        if path
            .components()
            .any(|component| matches!(component, Component::CurDir))
        {
            errors.push(format!(
                "manifest project '{}' target cannot contain '.': {}",
                project.id, project.target
            ));
        }
    }

    errors
}

fn is_allowed_source(source: &str) -> bool {
    let trimmed = source.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return false;
    }

    if let Some((scheme, _)) = trimmed.split_once("://") {
        return matches!(scheme, "https" | "ssh");
    }

    // SCP-like remote syntax: git@github.com:org/repo.git
    if !trimmed.contains("://") && trimmed.contains('@') && trimmed.contains(':') {
        return true;
    }

    // Local filesystem path (absolute or relative).
    Path::new(trimmed).components().count() > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_rejects_unsupported_source_scheme() {
        let manifest = Manifest {
            projects: vec![ManifestProject {
                id: "app".to_string(),
                source: "http://example.com/repo.git".to_string(),
                revision: None,
                target: "repos/app".to_string(),
                resolved_commit: None,
            }],
        };

        let errors = manifest_validation_errors(&manifest);
        assert!(
            errors.iter().any(|msg| msg.contains("unsupported source")),
            "expected unsupported source error, got: {errors:?}"
        );
    }

    #[test]
    fn manifest_accepts_local_source_path() {
        let manifest = Manifest {
            projects: vec![ManifestProject {
                id: "app".to_string(),
                source: "../seed-repo".to_string(),
                revision: Some("main".to_string()),
                target: "repos/app".to_string(),
                resolved_commit: None,
            }],
        };

        let errors = manifest_validation_errors(&manifest);
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorkspaceModel {
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) created_at: String,
    pub(crate) layout_version: u32,
    pub(crate) created_by: CreatedBy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) template: Option<TemplateRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) manifest: Option<Manifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CreatedBy {
    Manual,
    Template,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TemplateRef {
    #[serde(rename = "ref")]
    pub(crate) ref_path: String,
    pub(crate) digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) projects: Vec<ManifestProject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManifestProject {
    pub(crate) id: String,
    pub(crate) source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) revision: Option<String>,
    pub(crate) target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) resolved_commit: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateModel {
    version: u32,
    #[serde(default)]
    defaults: Option<TemplateDefaults>,
    #[serde(default)]
    manifest: Option<Manifest>,
}

#[derive(Debug, Clone, Deserialize)]
struct TemplateDefaults {
    #[serde(default)]
    layout_version: Option<u32>,
}
