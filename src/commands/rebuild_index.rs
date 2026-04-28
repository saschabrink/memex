use anyhow::Result;

use crate::config::MemexConfig;
use crate::{db, indexer, refresh};

const BATCH_SIZE: usize = 200;

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    let entries = cfg.all_blueprints();
    let total = entries.len();
    let embedder = indexer::Embedder::new()?;
    let mut indexed = 0;
    for chunk in entries.chunks(BATCH_SIZE) {
        let contents: Vec<String> = chunk
            .iter()
            .map(|(_, fp)| std::fs::read_to_string(fp))
            .collect::<std::io::Result<Vec<_>>>()?;
        let embeddings = embedder.embed_batch(contents.iter().collect::<Vec<_>>())?;
        for ((source, file_path), (content, emb)) in
            chunk.iter().zip(contents.iter().zip(embeddings.iter()))
        {
            let hash = refresh::sha256_hex(content.as_bytes());
            db::upsert(
                &mut conn,
                &cfg.blueprint_id(source, file_path),
                &cfg.extract_title(content),
                &file_path.to_string_lossy(),
                &source.name,
                content,
                &hash,
                emb,
            )?;
        }
        indexed += chunk.len();
        println!("Indexed {indexed}/{total}...");
    }
    println!("Done. Indexed {total} blueprints.");
    Ok(())
}
