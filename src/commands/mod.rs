use anyhow::{anyhow, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::config::{self, MemexConfig, Source};
use crate::{db, git, indexer, refresh};

pub mod agent_instructions;
pub mod broken_refs;
pub mod delete;
pub mod diff;
pub mod edit;
pub mod example_config;
pub mod hook_advice;
pub mod list;
pub mod move_;
pub mod read;
pub mod rebuild_index;
pub mod search;
pub mod sync;
pub mod versions;
pub mod write;

/// Escape a string for embedding inside a JSON string literal (RFC 8259).
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Error out if the source is marked `readonly = true`.
pub fn ensure_writable(source: &Source) -> Result<()> {
    if source.readonly {
        return Err(anyhow!(
            "source '{}' is read-only; writes are not permitted",
            source.name
        ));
    }
    Ok(())
}

pub fn read_stdin() -> Result<String> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    Ok(buf)
}

pub fn resolve_content(content: String) -> Result<String> {
    if content == "-" {
        read_stdin()
    } else {
        Ok(content)
    }
}

/// Resolve the git repo that should receive commits for changes to a source.
/// Walks up from the first existing path, or from the source's mount.
pub fn commit_repo_for(source: &Source, paths: &[&Path]) -> Result<PathBuf> {
    for p in paths {
        if let Some(repo) = config::find_enclosing_repo(p) {
            return Ok(repo);
        }
    }
    config::find_enclosing_repo(&source.mount)
        .ok_or_else(|| anyhow!("no git repo found for source '{}'", source.name))
}

/// Commit the given paths and, if the source has a remote, push best-effort.
pub fn commit_and_push(source: &Source, paths: &[&Path], message: &str) -> Result<()> {
    let repo = commit_repo_for(source, paths)?;
    git::commit(&repo, paths, message)?;
    if source.remote.is_some() {
        if let Err(e) = git::push(&repo) {
            eprintln!("warning: push of source '{}' failed: {e}", source.name);
        }
    }
    Ok(())
}

pub fn reindex_one(cfg: &MemexConfig, source: &Source, file_path: &Path) -> Result<()> {
    let mut conn = db::connect(&cfg.db_path())?;
    db::setup(&conn)?;
    let content = std::fs::read_to_string(file_path)?;
    let hash = refresh::sha256_hex(content.as_bytes());
    let embedder = indexer::Embedder::new()?;
    let emb = embedder.embed_one(&content)?;
    db::upsert(
        &mut conn,
        &cfg.blueprint_id(source, file_path),
        &cfg.extract_title(&content),
        &file_path.to_string_lossy(),
        &source.name,
        &content,
        &hash,
        &emb,
    )?;
    Ok(())
}
