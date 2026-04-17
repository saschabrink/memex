use anyhow::Result;

use crate::commands::commit_and_push;
use crate::config::MemexConfig;
use crate::{db, indexer, refresh};

pub fn run(cfg: &MemexConfig, old_id: &str, new_id: &str) -> Result<()> {
    let old = cfg.resolve_blueprint(old_id)?;
    let new = cfg.resolve_blueprint(new_id)?;

    if !old.file_path.exists() {
        println!("Blueprint '{old_id}' not found.");
        return Ok(());
    }
    if new.file_path.exists() {
        println!("Blueprint '{new_id}' already exists. Delete it first or choose a different ID.");
        return Ok(());
    }
    if let Some(parent) = new.file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = std::fs::read_to_string(&old.file_path)?;
    let same_source = old.source.name == new.source.name;

    if same_source {
        std::fs::rename(&old.file_path, &new.file_path)?;
        commit_and_push(
            old.source,
            &[&old.file_path, &new.file_path],
            &format!("Move blueprint: {old_id} → {new_id}"),
        )?;
    } else {
        std::fs::write(&new.file_path, &content)?;
        commit_and_push(
            new.source,
            &[&new.file_path],
            &format!("Add blueprint (moved from {old_id}): {new_id}"),
        )?;
        std::fs::remove_file(&old.file_path)?;
        commit_and_push(
            old.source,
            &[&old.file_path],
            &format!("Delete blueprint (moved to {new_id}): {old_id}"),
        )?;
    }

    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    db::del(&conn, old_id)?;
    let hash = refresh::sha256_hex(content.as_bytes());
    let embedder = indexer::Embedder::new()?;
    let emb = embedder.embed_one(&content)?;
    db::upsert(
        &mut conn,
        new_id,
        &cfg.extract_title(&content),
        &new.file_path.to_string_lossy(),
        &new.source.name,
        &content,
        &hash,
        &emb,
    )?;
    println!("Moved blueprint '{old_id}' → '{new_id}'");
    Ok(())
}
