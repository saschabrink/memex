use anyhow::Result;
use std::collections::BTreeSet;

use crate::config::MemexConfig;
use crate::links;

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let entries = cfg.all_blueprints();

    let mut all_slugs: BTreeSet<String> = BTreeSet::new();
    for (source, file_path) in &entries {
        all_slugs.insert(cfg.blueprint_id(source, file_path));
    }
    // Index of last path segment → true when at least one slug ends with it.
    // A bare ref `[[foo]]` (no slashes) is considered resolved if any known
    // slug's final segment equals `foo`.
    let mut tails: BTreeSet<&str> = BTreeSet::new();
    for slug in &all_slugs {
        let tail = slug.rsplit('/').next().unwrap_or(slug);
        tails.insert(tail);
    }

    let mut any = false;
    for (source, file_path) in &entries {
        let content = std::fs::read_to_string(file_path)?;
        let slugs = links::extract(&content);
        let unresolved: Vec<String> = slugs
            .into_iter()
            .filter(|s| {
                if all_slugs.contains(s) {
                    return false;
                }
                // bare ref (no slash) → accept if some slug has this tail
                if !s.contains('/') && tails.contains(s.as_str()) {
                    return false;
                }
                true
            })
            .collect();
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
