use anyhow::Result;

use crate::config::MemexConfig;
use crate::{db, refresh};

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    refresh::refresh(cfg, &mut conn)?;
    let rows = db::list_all(&conn, &cfg.all_source_names())?;
    for row in rows {
        let ro = cfg
            .source_by_name(&row.folder)
            .map(|s| s.readonly)
            .unwrap_or(false);
        let suffix = if ro { "  (read-only)" } else { "" };
        println!("{}  {}{}", row.id, row.title, suffix);
    }
    Ok(())
}
