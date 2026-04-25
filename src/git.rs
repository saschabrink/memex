use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub fn commit(repo_dir: &Path, paths: &[&Path], message: &str) -> Result<()> {
    let mut add = Command::new("git");
    add.arg("-C").arg(repo_dir).arg("add").arg("--");
    for p in paths {
        add.arg(p);
    }
    let status = add.status()?;
    if !status.success() {
        bail!("git add failed in {}", repo_dir.display());
    }

    let status = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("commit")
        .arg("-m")
        .arg(message)
        .status()?;
    if !status.success() {
        bail!("git commit failed in {}", repo_dir.display());
    }
    Ok(())
}

pub fn clone(remote: &str, target_dir: &Path) -> Result<()> {
    let status = Command::new("git")
        .arg("clone")
        .arg(remote)
        .arg(target_dir)
        .status()?;
    if !status.success() {
        bail!("git clone {remote} → {} failed", target_dir.display());
    }
    Ok(())
}

pub fn pull(repo_dir: &Path) -> Result<String> {
    let tracking = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .output()?;
    if !tracking.status.success() {
        return Ok("No upstream — skipping pull.".to_string());
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("pull")
        .output()?;
    if !out.status.success() {
        bail!(
            "git pull failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.contains("Already up to date") {
        Ok("Already up to date.".to_string())
    } else {
        Ok(stdout)
    }
}

/// Fast-forward-only pull. Fails loudly on divergence (never merges or rebases).
pub fn pull_ff_only(repo_dir: &Path) -> Result<String> {
    let tracking = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .output()?;
    if !tracking.status.success() {
        return Ok("No upstream — skipped.".to_string());
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["pull", "--ff-only"])
        .output()?;
    if !out.status.success() {
        bail!("{}", String::from_utf8_lossy(&out.stderr).trim());
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.contains("Already up to date") {
        Ok("up to date".to_string())
    } else {
        // Keep a short summary line for readable output.
        let first = stdout.lines().next().unwrap_or("").trim();
        if first.is_empty() {
            Ok("updated".to_string())
        } else {
            Ok(format!("updated ({first})"))
        }
    }
}

pub fn push(repo_dir: &Path) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["push", "-u", "origin", "HEAD"])
        .output()?;
    if !out.status.success() {
        bail!(
            "git push failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok("Pushed.".to_string())
}

pub fn log_file(repo_dir: &Path, file_path: &Path) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["log", "--format=%H\t%ai\t%s", "--"])
        .arg(file_path)
        .output()?;
    if !out.status.success() {
        bail!(
            "git log failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn show(repo_dir: &Path, hash: &str, file_path: &Path) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["show", hash, "--"])
        .arg(file_path)
        .output()?;
    if !out.status.success() {
        bail!(
            "git show failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn get_remote_url(repo_dir: &Path, remote: &str) -> Result<Option<String>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["remote", "get-url", remote])
        .output()?;
    if !out.status.success() {
        return Ok(None); // remote doesn't exist
    }
    Ok(Some(
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
    ))
}

pub fn set_remote_url(repo_dir: &Path, remote: &str, url: &str) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["remote", "set-url", remote, url])
        .status()?;
    if !status.success() {
        bail!(
            "git remote set-url {remote} failed in {}",
            repo_dir.display()
        );
    }
    Ok(())
}

pub fn is_inside_repo(dir: &Path) -> Result<bool> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    Ok(out.status.success())
}

pub fn init_with_gitignore(repo_dir: &Path, source_name: &str) -> Result<()> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .arg("init")
        .status()?;
    if !status.success() {
        bail!("git init failed in {}", repo_dir.display());
    }
    let gitignore = repo_dir.join(".gitignore");
    if !gitignore.exists() {
        std::fs::write(&gitignore, ".db/\n")
            .with_context(|| format!("writing {}", gitignore.display()))?;
    }
    commit(
        repo_dir,
        &[&gitignore],
        &format!("Init memex source: {source_name}"),
    )?;
    Ok(())
}
