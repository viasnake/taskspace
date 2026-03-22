use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn run_command(program: &str, args: &[String]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to execute command: {program}"))?;

    if !status.success() {
        bail!("command failed ({program}): {status}");
    }

    Ok(())
}

pub fn run_command_capture(program: &str, args: &[String]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to execute command: {program}"))?;

    if !output.status.success() {
        bail!(
            "command failed ({program}): {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
