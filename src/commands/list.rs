use anyhow::Result;

use crate::config::MemexConfig;
use crate::{db, refresh};

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    refresh::refresh(cfg, &mut conn)?;
    let rows = db::list_all(&conn, &cfg.all_folders())?;
    for row in rows {
        println!("{}  {}", row.id, row.title);
    }
    Ok(())
}
