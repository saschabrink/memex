use anyhow::Result;

use crate::commands::commit_repo_for;
use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig, id: &str) -> Result<()> {
    let resolved = cfg.resolve_blueprint(id)?;
    if !resolved.file_path.exists() {
        return Ok(());
    }
    let repo = commit_repo_for(resolved.source, &[&resolved.file_path])?;
    let raw = git::log_file(&repo, &resolved.file_path)?;
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() < 3 {
            continue;
        }
        println!("{}  {}  {}", parts[0], parts[1], parts[2]);
    }
    Ok(())
}
