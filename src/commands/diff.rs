use anyhow::Result;

use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig, id: &str, hash: &str) -> Result<()> {
    let resolved = cfg.resolve_blueprint(id)?;
    let out = git::show(&resolved.source.path, hash, &resolved.file_path)?;
    print!("{out}");
    Ok(())
}
