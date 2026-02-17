use anyhow::{Context, Error, anyhow};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_git_changes(path: &str, since_commit: &str) -> anyhow::Result<Vec<PathBuf>, Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("diff")
        .arg("--name-only")
        .arg(since_commit)
        .arg("HEAD")
        .output()
        .context("Git command could not run. Check git is present?")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Git error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let files = stdout
        .lines()
        .map(|line| Path::new(path).join(line))
        .filter(|p| p.is_file())
        .collect();

    Ok(files)
}
