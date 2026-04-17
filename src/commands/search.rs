use anyhow::Result;

use crate::config::MemexConfig;
use crate::{db, indexer, refresh};

pub fn run(cfg: &MemexConfig, query: &str, limit: usize) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    refresh::refresh(cfg, &mut conn)?;
    let embedder = indexer::Embedder::new()?;
    let q = embedder.embed_one(query)?;
    let results = db::search(&conn, &q, &cfg.all_source_names(), limit)?;
    for r in results {
        let ro = cfg
            .source_by_name(&r.folder)
            .map(|s| s.readonly)
            .unwrap_or(false);
        let suffix = if ro { "  (read-only)" } else { "" };
        println!("{}  [{:.3}]  {}{}", r.id, r.distance, r.title, suffix);
    }
    Ok(())
}
