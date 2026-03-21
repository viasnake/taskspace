use std::path::Path;

use anyhow::{Result, anyhow};
use taskspace_core::TaskspaceError;
use taskspace_infra_fs::{create_dir, run_command, run_command_capture};

use crate::map_infra_error;
use crate::template::Manifest;

pub fn clone_manifest_projects(session_dir: &Path, manifest: &mut Manifest) -> Result<()> {
    for project in &mut manifest.projects {
        let target_dir = session_dir.join(&project.target);
        let target_dir_text = target_dir.display().to_string();
        if target_dir.exists() {
            return Err(anyhow!(TaskspaceError::Conflict(format!(
                "manifest target already exists: {}",
                project.target
            ))));
        }

        if let Some(parent) = target_dir.parent() {
            create_dir(parent).map_err(map_infra_error)?;
        }

        run_git(
            &[
                "clone".to_string(),
                "--".to_string(),
                project.source.clone(),
                target_dir_text.clone(),
            ],
            &format!("failed to clone project '{}'", project.id),
        )?;

        if let Some(revision) = &project.revision {
            run_git(
                &[
                    "-C".to_string(),
                    target_dir_text.clone(),
                    "checkout".to_string(),
                    revision.clone(),
                ],
                &format!(
                    "failed to checkout revision '{}' for project '{}'",
                    revision, project.id
                ),
            )?;
        }

        let resolved_commit = run_git_capture(
            &[
                "-C".to_string(),
                target_dir_text,
                "rev-parse".to_string(),
                "HEAD".to_string(),
            ],
            &format!("failed to resolve HEAD commit for project '{}'", project.id),
        )?;
        project.resolved_commit = Some(resolved_commit);
    }

    Ok(())
}

fn run_git(args: &[String], context: &str) -> Result<()> {
    run_command("git", args)
        .map_err(|err| anyhow!(TaskspaceError::ExternalCommand(format!("{context}: {err}"))))
}

fn run_git_capture(args: &[String], context: &str) -> Result<String> {
    run_command_capture("git", args)
        .map_err(|err| anyhow!(TaskspaceError::ExternalCommand(format!("{context}: {err}"))))
}
