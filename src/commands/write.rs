use anyhow::Result;

use crate::commands::{commit_and_push, ensure_writable, reindex_one, resolve_content};
use crate::config::MemexConfig;

pub fn run(cfg: &MemexConfig, id: &str, content_arg: &str) -> Result<()> {
    let content = resolve_content(content_arg.to_string())?;
    let resolved = cfg.resolve_blueprint(id)?;
    ensure_writable(resolved.source)?;
    if let Some(parent) = resolved.file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let existed = resolved.file_path.exists();
    std::fs::write(&resolved.file_path, &content)?;

    let verb_past = if existed { "Rewrite" } else { "Add" };
    commit_and_push(
        resolved.source,
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
