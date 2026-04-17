use anyhow::Result;
use std::collections::BTreeSet;

use crate::config::MemexConfig;
use crate::links;

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let entries = cfg.all_blueprints();

    let mut all_stems: BTreeSet<String> = BTreeSet::new();
    let mut shared_stems: BTreeSet<String> = BTreeSet::new();
    for (source, file_path) in &entries {
        let id = cfg.blueprint_id(source, file_path);
        let stem = id.rsplit('/').next().unwrap_or(&id).to_string();
        all_stems.insert(stem.clone());
        if !source.project {
            shared_stems.insert(stem);
        }
    }

    let mut any = false;
    for (source, file_path) in &entries {
        let content = std::fs::read_to_string(file_path)?;
        let slugs = links::extract(&content);
        let mut unresolved: Vec<String> = Vec::new();
        for slug in slugs {
            if !all_stems.contains(&slug) {
                unresolved.push(slug);
            } else if !source.project && !shared_stems.contains(&slug) {
                unresolved.push(format!(
                    "{slug} (project-only reference not allowed in shared source)"
                ));
            }
        }
        if !unresolved.is_empty() {
            any = true;
            println!(
                "{}: {}",
                cfg.blueprint_id(source, file_path),
                unresolved.join(", ")
            );
        }
    }
    if !any {
        println!("No broken references found.");
    }
    Ok(())
}
