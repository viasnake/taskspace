use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use taskspace_core::TaskspaceError;

pub fn default_sessions_root() -> Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(home.join("taskspace").join("sessions"))
}

pub fn global_skills_paths() -> Result<Vec<PathBuf>> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(vec![
        home.join(".taskspace").join("SKILLS.md"),
        home.join(".config").join("taskspace").join("SKILLS.md"),
    ])
}

pub fn archive_root(root_dir: &Path) -> Result<PathBuf> {
    let parent = root_dir.parent().ok_or_else(|| {
        anyhow!(TaskspaceError::Internal(
            "cannot resolve archive root from sessions root".to_string()
        ))
    })?;
    Ok(parent.join("archive"))
}
