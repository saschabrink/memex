use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

use crate::config::{MemexConfig, Source};
use crate::db;
use crate::indexer::Embedder;

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn refresh(cfg: &MemexConfig, conn: &mut rusqlite::Connection) -> Result<()> {
    let disk = cfg.all_blueprints();

    let mut disk_by_id: HashMap<String, (&Source, std::path::PathBuf, String, String)> =
        HashMap::with_capacity(disk.len());
    for (source, file_path) in &disk {
        let content = std::fs::read_to_string(file_path)?;
        let hash = sha256_hex(content.as_bytes());
        let id = cfg.blueprint_id(source, file_path);
        disk_by_id.insert(id, (*source, file_path.clone(), content, hash));
    }

    let indexed = db::index_state(conn)?;
    let indexed_ids: HashSet<&str> = indexed.iter().map(|(id, _)| id.as_str()).collect();
    let indexed_hashes: HashMap<&str, &str> =
        indexed.iter().map(|(id, h)| (id.as_str(), h.as_str())).collect();

    for (id, _) in &indexed {
        if !disk_by_id.contains_key(id) {
            db::del(conn, id)?;
        }
    }

    let mut to_embed: Vec<(String, String, String)> = Vec::new();
    for (id, (_, _, content, hash)) in &disk_by_id {
        let needs = match indexed_hashes.get(id.as_str()) {
            None => true,
            Some(old) => old != hash,
        };
        if needs {
            to_embed.push((id.clone(), content.clone(), hash.clone()));
        }
    }

    if to_embed.is_empty() {
        let _ = indexed_ids;
        return Ok(());
    }

    let embedder = Embedder::new()?;
    let texts: Vec<&str> = to_embed.iter().map(|(_, c, _)| c.as_str()).collect();
    let embeddings = embedder.embed_batch(texts)?;

    for ((id, content, hash), emb) in to_embed.iter().zip(embeddings.iter()) {
        let (source, file_path, _, _) = disk_by_id.get(id).unwrap();
        db::upsert(
            conn,
            id,
            &cfg.extract_title(content),
            &file_path.to_string_lossy(),
            &source.name,
            content,
            hash,
            emb,
        )?;
    }

    Ok(())
}
