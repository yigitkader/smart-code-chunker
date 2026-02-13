use anyhow::{Context, Error, anyhow};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_files(path: &str, since: &Option<String>) -> Result<Vec<PathBuf>, Error> {
    let files: Vec<PathBuf> = if let Some(commit_hash) = &since {
        println!("Smart chunker started with git hash: {}", commit_hash);
        get_git_changes(path, commit_hash)?
    } else {
        println!("Smart chunker started with full scan");
        let walker = WalkBuilder::new(path).standard_filters(true).build();
        walker
            .filter_map(|r| r.ok().map(|e| e.into_path()).filter(|p| p.is_file()))
            .collect()
    };
    Ok(files)
}

fn get_git_changes(path: &str, since_commit: &str) -> Result<Vec<PathBuf>, Error> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("diff")
        .arg("--name-only")
        .arg(since_commit)
        .arg("HEAD")
        .output()
        .context("Git komutu çalıştırılamadı. Git yüklü mü?")?;

    if !output.status.success() {
        return Err(anyhow!(
            "Git hatası: {}",
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
