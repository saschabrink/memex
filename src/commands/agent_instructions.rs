//! `memex agent-instructions` — emits a generic usage brief for LLM agents.
//!
//! Two output modes:
//! - human (default): plain markdown to stdout, suitable for `memex
//!   agent-instructions | pbcopy` or manual inspection.
//! - `--claude-hook`: JSON envelope for a Claude Code SessionStart hook. The
//!   same markdown is delivered as `additionalContext`.
//!
//! Kept project-agnostic on purpose: project-specific context belongs in
//! CLAUDE.md / AGENTS.md. This command only describes how to *use* memex.
//!
//! If a `memex.toml` is present in the cwd, an auto-generated
//! "This project's sources" section is appended. This replaces the
//! handwritten source list projects used to maintain in their CLAUDE.md.

use anyhow::Result;
use std::fmt::Write;

use crate::commands::json_escape;
use crate::config::{self, MemexConfig, Source};

const INSTRUCTIONS: &str = r#"# Using memex

memex is a semantic blueprint index. Blueprints are markdown documents with
stable slug IDs (like `shared/phoenix-liveview/liveview`). You use them as
reference material — architecture patterns, coding conventions, gotchas.

memex is a **CLI tool**. Invoke it via your shell-exec tool (Bash, etc.)
like any other command — there is no MCP server and none is planned.

## Core commands

- `memex search <query> [--limit N]` — semantic search. Free-form queries
  describing the problem work better than exact slug guesses.
- `memex read <slug>` — read a blueprint by slug. Use this after search finds
  a relevant one, or when you already know the slug.
- `memex list` — enumerate all blueprints (rarely useful except for sanity
  checks).
- `memex write <slug> <content>` / `memex write <slug> -` (stdin) — create
  or overwrite. Commits to git automatically. Writes to read-only sources
  are rejected.
- `memex edit <slug> <old> <new>` — first-occurrence literal replace.
- `memex broken-refs` — list blueprints with dangling `[[slug]]` references.

Because memex is a CLI, you can combine it with shell plumbing the same
way you would with `grep` or `jq`. For example, dumping a batch of notes
from a pipeline into memex:

    for f in notes/*.md; do
      slug="scratch/$(basename "$f" .md)"
      memex write "$slug" - < "$f"
    done

## When to reach for memex

- Starting a new file that matches a recognizable pattern (LiveView, context
  module, Oban worker, schema migration, etc.) — pre-write hooks may already
  inject the right blueprint for you; otherwise search.
- Debugging a subtle behavior that looks like it might have a convention
  attached — search before asking the user.
- User references a pattern by name — look it up before guessing.

## What NOT to do

- Don't paraphrase blueprint content from memory. Read the current version.
- Don't skip a pre-write hook's advice just because the blueprint name sounds
  familiar. Version drift happens.
- Don't write to read-only sources (they'll reject the write). Suggest the
  user edit the upstream repo if content needs changing.

## Editing memex.toml

If the user asks you to add a source, change slugs, or otherwise edit
`memex.toml`, run `memex example-config` first. It prints a fully
annotated example covering every option and copy-paste starting points
for common ecosystems (Elixir/Hex, Node `node_modules`, Ruby gems,
Rust `cargo vendor`, Python `site-packages`, Obsidian vaults). Use it
instead of guessing field names — the TOML schema is strict and typos
fail to load.
"#;

pub fn run(claude_hook: bool) -> Result<()> {
    let mut body = String::from(INSTRUCTIONS);

    // If a memex.toml is present in cwd, append auto-generated project info.
    // Silent on failure — the generic brief must work without a config.
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(cfg) = config::load(&cwd, None) {
            body.push_str(&project_section(&cfg));
        }
    }

    if claude_hook {
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"{}\"}}}}",
            json_escape(&body),
        );
    } else {
        print!("{body}");
    }
    Ok(())
}

/// Render a human-readable description of each source in the config.
/// Appended to the generic brief when a `memex.toml` is found in cwd.
fn project_section(cfg: &MemexConfig) -> String {
    let mut out = String::new();
    out.push_str("\n## This project's sources\n\n");
    out.push_str(&format!("Project: `{}`.\n\n", cfg.project_name));
    for src in &cfg.sources {
        let _ = writeln!(out, "- {}", source_line(src, &cfg.project_root));
    }
    if !cfg.also_scan.is_empty() {
        out.push_str(&format!(
            "\nAlso scanned for `[[slug]]` refs (not indexed): {}.\n",
            cfg.also_scan
                .iter()
                .map(|g| format!("`{g}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out
}

fn source_line(src: &Source, project_root: &std::path::Path) -> String {
    // Mount as a project-root-relative path when possible.
    let mount_disp = src
        .mount
        .strip_prefix(project_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| src.mount.display().to_string());
    let mount_disp = if mount_disp.is_empty() {
        ".".to_string()
    } else {
        mount_disp
    };

    let mut attrs: Vec<String> = Vec::new();
    if let Some(remote) = &src.remote {
        attrs.push(format!("synced from `{remote}`"));
    }
    if src.readonly {
        attrs.push("read-only".to_string());
    }
    if let Some(idx) = &src.index_filename {
        attrs.push(format!("index_filename `{idx}`"));
    }
    if src.prefix != src.name {
        attrs.push(if src.prefix.is_empty() {
            "bare slugs (no prefix)".to_string()
        } else {
            format!("prefix `{}`", src.prefix)
        });
    }

    let attrs_str = if attrs.is_empty() {
        String::new()
    } else {
        format!(" — {}", attrs.join(", "))
    };

    format!("`{}` — mounts `{}`{}", src.name, mount_disp, attrs_str)
}
