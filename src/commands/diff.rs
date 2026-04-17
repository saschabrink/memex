use anyhow::Result;

use crate::commands::commit_repo_for;
use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig, id: &str, hash: &str) -> Result<()> {
    let resolved = cfg.resolve_blueprint(id)?;
    let repo = commit_repo_for(resolved.source, &[&resolved.file_path])?;
    let out = git::show(&repo, hash, &resolved.file_path)?;
    print!("{out}");
    Ok(())
}
