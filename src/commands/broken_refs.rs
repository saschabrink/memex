use anyhow::Result;
use std::collections::BTreeSet;

use crate::config::MemexConfig;
use crate::hooks;
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
    // Also check hook blueprint references. Bare (no-slash) ids fall back to
    // the same tail-match rule as `[[slug]]` refs in blueprint bodies.
    let set = hooks::load(cfg)?;
    let check_hook = |hook: &hooks::Hook, event: &str, any: &mut bool| {
        let unresolved: Vec<&str> = hook
            .blueprints
            .iter()
            .filter(|s| {
                if all_slugs.contains(s.as_str()) {
                    return false;
                }
                if !s.contains('/') && tails.contains(s.as_str()) {
                    return false;
                }
                true
            })
            .map(|s| s.as_str())
            .collect();
        if !unresolved.is_empty() {
            *any = true;
            let origin = match &hook.source {
                Some(name) => format!("source '{name}'"),
                None => "project".to_string(),
            };
            let label = match (&hook.pattern_src, &hook.content_pattern_src) {
                (Some(p), Some(c)) => format!("path={p} content={c}"),
                (Some(p), None) => p.clone(),
                (None, Some(c)) => format!("content={c}"),
                (None, None) => "(unnamed)".to_string(),
            };
            println!(
                "hook [{event}] {label} in {origin}: {}",
                unresolved.join(", ")
            );
        }
    };
    for hook in &set.pre_write {
        check_hook(hook, "pre-write", &mut any);
    }
    for hook in &set.post_write {
        check_hook(hook, "post-write", &mut any);
    }

    // Scan external files listed in `also_scan` (e.g. CLAUDE.md, AGENTS.md).
    // These files reference blueprints but are not indexed themselves.
    for file in cfg.also_scan_files()? {
        let content = match std::fs::read_to_string(&file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let slugs = links::extract(&content);
        let unresolved: Vec<String> = slugs
            .into_iter()
            .filter(|s| {
                if all_slugs.contains(s) {
                    return false;
                }
                if !s.contains('/') && tails.contains(s.as_str()) {
                    return false;
                }
                true
            })
            .collect();
        if !unresolved.is_empty() {
            any = true;
            let rel = file
                .strip_prefix(&cfg.project_root)
                .unwrap_or(&file)
                .display();
            println!("{rel}: {}", unresolved.join(", "));
        }
    }

    if !any {
        println!("No broken references found.");
    }
    Ok(())
}
