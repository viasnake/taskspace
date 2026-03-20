use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub fn create_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .with_context(|| format!("failed to create directory: {}", path.display()))
}

pub fn write_file(path: &Path, content: &str) -> Result<()> {
    create_dir(path.parent().unwrap_or(Path::new(".")))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("failed to create file: {}", path.display()))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("failed to write file: {}", path.display()))
}

pub fn list_directories(path: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    if !path.exists() {
        return Ok(names);
    }

    for entry in
        fs::read_dir(path).with_context(|| format!("failed to read: {}", path.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            names.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    names.sort();
    Ok(names)
}

pub fn canonicalize_if_exists(raw: &str) -> Option<PathBuf> {
    let path = PathBuf::from(raw);
    if path.exists() {
        return fs::canonicalize(path).ok();
    }
    None
}

pub fn move_dir(src: &Path, dst: &Path) -> Result<()> {
    fs::rename(src, dst).with_context(|| {
        format!(
            "failed to move directory from {} to {}",
            src.display(),
            dst.display()
        )
    })
}

pub fn remove_dir_all(path: &Path) -> Result<()> {
    fs::remove_dir_all(path)
        .with_context(|| format!("failed to remove directory: {}", path.display()))
}

pub fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("failed to read file: {}", path.display()))
}
