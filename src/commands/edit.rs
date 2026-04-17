use anyhow::Result;

use crate::commands::reindex_one;
use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig, id: &str, old: &str, new: &str) -> Result<()> {
    let resolved = cfg.resolve_blueprint(id)?;
    if !resolved.file_path.exists() {
        println!("Blueprint '{id}' not found. Use write to create it.");
        return Ok(());
    }
    let content = std::fs::read_to_string(&resolved.file_path)?;
    if !content.contains(old) {
        println!("old_string not found in blueprint '{id}'.");
        return Ok(());
    }
    let updated = content.replacen(old, new, 1);
    std::fs::write(&resolved.file_path, &updated)?;
    git::commit(
        &resolved.source.path,
        &[&resolved.file_path],
        &format!("Update blueprint: {id}"),
    )?;
    reindex_one(cfg, resolved.source, &resolved.file_path)?;
    println!("Updated blueprint '{id}'");
    Ok(())
}
