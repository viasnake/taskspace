use std::path::Path;

use anyhow::{Result, anyhow};
use taskspace_core::TaskspaceError;
use taskspace_infra_fs::{create_dir, run_command, run_command_capture};

use crate::map_infra_error;
use crate::template::Manifest;

pub fn clone_manifest_projects(session_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    let plan = plan_manifest_clones(session_dir, manifest)?;
    for (project, planned) in manifest.projects.iter_mut().zip(plan) {
        clone_project(&planned)?;
        checkout_revision_if_requested(&planned)?;
        project.resolved_commit = Some(resolve_head_commit(&planned)?);
    }

    Ok(())
}

struct PlannedClone {
    id: String,
    source: String,
    revision: Option<String>,
    target_dir_text: String,
}

fn plan_manifest_clones(session_dir: &Path, manifest: &Manifest) -> Result<Vec<PlannedClone>> {
    let mut planned = Vec::with_capacity(manifest.projects.len());
    for project in &manifest.projects {
        let target_dir = session_dir.join(&project.target);
        if target_dir.exists() {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "manifest target already exists: {}",
                project.target
            ))));
        }

        if let Some(parent) = target_dir.parent() {
            create_dir(parent).map_err(map_infra_error)?;
        }

        planned.push(PlannedClone {
            id: project.id.clone(),
            source: project.source.clone(),
            revision: project.revision.clone(),
            target_dir_text: target_dir.display().to_string(),
        });
    }

    Ok(planned)
}

fn clone_project(planned: &PlannedClone) -> Result<()> {
    run_git(
        &[
            "clone".to_string(),
            "--".to_string(),
            planned.source.clone(),
            planned.target_dir_text.clone(),
        ],
        &format!("failed to clone project '{}'", planned.id),
    )
}

fn checkout_revision_if_requested(planned: &PlannedClone) -> Result<()> {
    if let Some(revision) = &planned.revision {
        run_git(
            &[
                "-C".to_string(),
                planned.target_dir_text.clone(),
                "checkout".to_string(),
                revision.clone(),
            ],
            &format!(
                "failed to checkout revision '{}' for project '{}'",
                revision, planned.id
            ),
        )?;
    }

    Ok(())
}

fn resolve_head_commit(planned: &PlannedClone) -> Result<String> {
    run_git_capture(
        &[
            "-C".to_string(),
            planned.target_dir_text.clone(),
            "rev-parse".to_string(),
            "HEAD".to_string(),
        ],
        &format!("failed to resolve HEAD commit for project '{}'", planned.id),
    )
}

fn run_git(args: &[String], context: &str) -> Result<()> {
    run_command("git", args)
        .map_err(|err| anyhow!(TaskspaceError::ExternalCommand(format!("{context}: {err}"))))
}

fn run_git_capture(args: &[String], context: &str) -> Result<String> {
    run_command_capture("git", args)
        .map_err(|err| anyhow!(TaskspaceError::ExternalCommand(format!("{context}: {err}"))))
}
