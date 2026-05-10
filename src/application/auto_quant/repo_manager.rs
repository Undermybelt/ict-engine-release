use std::path::Path;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};

pub fn git_output(cwd: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut command = Command::new("git");
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn is_git_repo(dir: &Path) -> bool {
    git_output(Some(dir), &["rev-parse", "--git-dir"]).is_ok()
}

pub fn current_commit(dir: &Path) -> Result<String> {
    git_output(Some(dir), &["rev-parse", "HEAD"])
}

pub fn upstream_commit(dir: &Path, tracked_branch: &str) -> Result<String> {
    let reference = format!("refs/heads/{tracked_branch}");
    let line = git_output(Some(dir), &["ls-remote", "origin", &reference])?;
    let commit = line
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("no upstream commit found for '{}'", tracked_branch))?;
    Ok(commit.to_string())
}
