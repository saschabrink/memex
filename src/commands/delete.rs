use anyhow::Result;

use crate::commands::{commit_and_push, ensure_writable};
use crate::config::MemexConfig;
use crate::db;

pub fn run(cfg: &MemexConfig, id: &str) -> Result<()> {
    let resolved = cfg.resolve_blueprint(id)?;
    ensure_writable(resolved.source)?;
    if !resolved.file_path.exists() {
        println!("Blueprint '{id}' not found.");
        return Ok(());
    }
    std::fs::remove_file(&resolved.file_path)?;
    commit_and_push(
        resolved.source,
        &[&resolved.file_path],
        &format!("Delete blueprint: {id}"),
    )?;
    let conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    db::del(&conn, id)?;
    println!("Deleted blueprint '{id}'");
    Ok(())
}
