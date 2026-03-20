use std::path::Path;

use anyhow::Result;
use chrono::Utc;
use serde::Serialize;
use taskspace_core::{RepoSpec, WORKSPACE_SCHEMA_VERSION};
use taskspace_infra_fs::write_file;

use crate::map_infra_error;
use crate::repo_import::redact_source;
use crate::spec;

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

pub fn write_templates(session_dir: &Path, session_name: &str, repos: &[RepoSpec]) -> Result<()> {
    let session_md = format!(
        "# Session: {session_name}\n\n## Objective\nDescribe the task objective.\n\n## Scope\nDescribe the scope and boundaries.\n"
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

    let workspace = VsCodeWorkspace {
        folders: build_workspace_folders(repos),
        settings: VsCodeSettings {
            files_exclude: serde_json::json!({
                "**/.git": true,
                "**/node_modules": true,
            }),
        },
    };
    write_relative_file(
        session_dir,
        "workspace.code-workspace",
        &serde_json::to_string_pretty(&workspace)?,
    )?;

    let model = WorkspaceModel {
        version: WORKSPACE_SCHEMA_VERSION,
        name: session_name.to_string(),
        created_at: Utc::now().to_rfc3339(),
        repos: repos
            .iter()
            .map(|repo| WorkspaceRepo {
                name: repo.name.clone(),
                source: redact_source(&repo.source),
            })
            .collect(),
    };
    write_relative_file(
        session_dir,
        "workspace.yaml",
        &serde_yaml::to_string(&model)?,
    )?;

    Ok(())
}

fn write_relative_file(base: &Path, rel: &str, content: &str) -> Result<()> {
    write_file(&base.join(rel), content).map_err(map_infra_error)
}

fn build_workspace_folders(repos: &[RepoSpec]) -> Vec<VsCodeFolder> {
    let mut folders: Vec<VsCodeFolder> = repos
        .iter()
        .map(|repo| VsCodeFolder {
            name: repo.name.clone(),
            path: format!("repos/{}", repo.name),
        })
        .collect();

    folders.push(VsCodeFolder {
        name: "context".to_string(),
        path: "context".to_string(),
    });
    folders.push(VsCodeFolder {
        name: "notes".to_string(),
        path: "notes".to_string(),
    });

    folders
}

#[derive(Serialize)]
struct VsCodeWorkspace {
    folders: Vec<VsCodeFolder>,
    settings: VsCodeSettings,
}

#[derive(Serialize)]
struct VsCodeFolder {
    name: String,
    path: String,
}

#[derive(Serialize)]
struct VsCodeSettings {
    #[serde(rename = "files.exclude")]
    files_exclude: serde_json::Value,
}

#[derive(Serialize)]
struct WorkspaceModel {
    version: u32,
    name: String,
    created_at: String,
    repos: Vec<WorkspaceRepo>,
}

#[derive(Serialize)]
struct WorkspaceRepo {
    name: String,
    source: String,
}
