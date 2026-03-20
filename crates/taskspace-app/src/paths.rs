use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use taskspace_core::TaskspaceError;

/// Returns the taskspace root directory: `~/taskspace/`.
/// Session directories are created directly under this root.
pub fn default_sessions_root() -> Result<PathBuf> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(home.join("taskspace"))
}

pub fn global_skills_paths() -> Result<Vec<PathBuf>> {
    let home = home::home_dir()
        .ok_or_else(|| anyhow!(TaskspaceError::Internal("cannot resolve HOME".to_string())))?;
    Ok(vec![
        home.join(".taskspace").join("SKILLS.md"),
        home.join(".config").join("taskspace").join("SKILLS.md"),
    ])
}

/// Returns the archive root directory: `<taskspace_root>/.archive/`.
pub fn archive_root(root_dir: &Path) -> Result<PathBuf> {
    Ok(root_dir.join(".archive"))
}
