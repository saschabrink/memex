use anyhow::Result;

use crate::config::MemexConfig;
use crate::db;

pub fn run(cfg: &MemexConfig, id: &str) -> Result<()> {
    let conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    match db::get(&conn, id)? {
        Some(row) => {
            if row.content.trim_start().starts_with("# ") {
                print!("{}", row.content);
                if !row.content.ends_with('\n') {
                    println!();
                }
            } else {
                println!("# {}\n\n{}", row.title, row.content);
            }
            Ok(())
        }
        None => {
            eprintln!("Blueprint not found: {id}");
            std::process::exit(1);
        }
    }
}
