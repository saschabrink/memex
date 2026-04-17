use anyhow::Result;

use crate::commands::{reindex_one, resolve_content};
use crate::config::MemexConfig;
use crate::git;

pub fn run(cfg: &MemexConfig, id: &str, content_arg: &str) -> Result<()> {
    let content = resolve_content(content_arg.to_string())?;
    let resolved = cfg.resolve_blueprint(id)?;
    if let Some(parent) = resolved.file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existed = resolved.file_path.exists();
    std::fs::write(&resolved.file_path, &content)?;

    let verb_past = if existed { "Rewrite" } else { "Add" };
    git::commit(
        &resolved.source.path,
        &[&resolved.file_path],
        &format!("{verb_past} blueprint: {id}"),
    )?;
    reindex_one(cfg, resolved.source, &resolved.file_path)?;

    println!(
        "{} blueprint '{id}'",
        if existed { "Rewrote" } else { "Added" }
    );
    Ok(())
}
