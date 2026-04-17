use anyhow::Result;
use std::io::Read;
use std::path::Path;

use crate::config::{MemexConfig, Source};
use crate::{db, indexer, refresh};

pub mod broken_refs;
pub mod delete;
pub mod diff;
pub mod edit;
pub mod list;
pub mod move_;
pub mod read;
pub mod rebuild_index;
pub mod search;
pub mod sync;
pub mod versions;
pub mod write;

pub fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

pub fn resolve_content(content: String) -> Result<String> {
    if content == "-" {
        read_stdin()
    } else {
        Ok(content)
    }
}

pub fn reindex_one(cfg: &MemexConfig, source: &Source, file_path: &Path) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    let content = std::fs::read_to_string(file_path)?;
    let hash = refresh::sha256_hex(content.as_bytes());
    let embedder = indexer::Embedder::new()?;
    let emb = embedder.embed_one(&content)?;
    db::upsert(
        &mut conn,
        &cfg.blueprint_id(source, file_path),
        &cfg.extract_title(&content),
        &file_path.to_string_lossy(),
        &cfg.blueprint_folder(source, file_path),
        &content,
        &hash,
        &emb,
    )?;
    Ok(())
}
