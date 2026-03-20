use std::path::Path;

use anyhow::{Result, anyhow};
use taskspace_core::{RepoSpec, TaskspaceError};
use taskspace_infra_fs::{canonicalize_if_exists, run_command};
use url::Url;

pub fn import_repos(session_dir: &Path, repos: &[RepoSpec]) -> Result<()> {
    for repo in repos {
        let dest = session_dir.join("repos").join(&repo.name);
        let source = if let Some(path) = canonicalize_if_exists(&repo.source) {
            path.display().to_string()
        } else {
            repo.source.clone()
        };

        if source.starts_with('-') {
            return Err(anyhow!(TaskspaceError::Usage(format!(
                "repo source for '{}' cannot start with '-'",
                repo.name
            ))));
        }

        let args = vec![
            "clone".to_string(),
            "--".to_string(),
            source,
            dest.display().to_string(),
        ];
        if let Err(err) = run_command("git", &args) {
            return Err(anyhow!(TaskspaceError::ExternalCommand(format!(
                "failed to import repo '{}={}': {err}",
                repo.name,
                redact_source(&repo.source)
            ))));
        }
    }

    Ok(())
}

pub fn redact_source(raw: &str) -> String {
    if let Ok(mut url) = Url::parse(raw) {
        let _ = url.set_username("");
        let _ = url.set_password(None);
        url.set_query(None);
        url.set_fragment(None);
        return url.to_string();
    }

    raw.to_string()
}
