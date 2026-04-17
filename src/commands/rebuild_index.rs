use anyhow::Result;

use crate::config::MemexConfig;
use crate::{db, indexer, refresh};

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    let entries = cfg.all_blueprints();
    let contents: Vec<String> = entries
        .iter()
        .map(|(_, fp)| std::fs::read_to_string(fp))
        .collect::<std::io::Result<Vec<_>>>()?;
    let embedder = indexer::Embedder::new()?;
    let embeddings = embedder.embed_batch(contents.iter().collect::<Vec<_>>())?;
    for ((source, file_path), (content, emb)) in
        entries.iter().zip(contents.iter().zip(embeddings.iter()))
    {
        let hash = refresh::sha256_hex(content.as_bytes());
        db::upsert(
            &mut conn,
            &cfg.blueprint_id(source, file_path),
            &cfg.extract_title(content),
            &file_path.to_string_lossy(),
            &cfg.blueprint_folder(source, file_path),
            content,
            &hash,
            emb,
        )?;
    }
    println!("Indexed {} blueprints.", entries.len());
    Ok(())
}
