use anyhow::Result;

use crate::config::MemexConfig;
use crate::{db, git};

pub fn run(cfg: &MemexConfig, id: &str) -> Result<()> {
    let resolved = cfg.resolve_blueprint(id)?;
    if !resolved.file_path.exists() {
        println!("Blueprint '{id}' not found.");
        return Ok(());
    }
    std::fs::remove_file(&resolved.file_path)?;
    git::commit(
        &resolved.source.path,
        &[&resolved.file_path],
        &format!("Delete blueprint: {id}"),
    )?;
    let conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    db::del(&conn, id)?;
    println!("Deleted blueprint '{id}'");
    Ok(())
}
