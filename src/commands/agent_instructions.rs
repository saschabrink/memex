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

use anyhow::Result;

use crate::commands::json_escape;

const INSTRUCTIONS: &str = r#"# Using memex

memex is a semantic blueprint index. Blueprints are markdown documents with
stable slug IDs (like `shared/phoenix-liveview/liveview`). You use them as
reference material — architecture patterns, coding conventions, gotchas.

## Tool names (via MCP)

- `mcp__memex__search_blueprints` — semantic search. Use free-form queries
  describing the problem, not exact slug guesses.
- `mcp__memex__read_blueprint` — read a blueprint by slug ID. Use this after
  search finds a relevant one, or when you already know the slug.
- `mcp__memex__list_blueprints` — enumerate all blueprints (rarely useful
  except for sanity checks).

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

## CLI fallbacks

If MCP tools aren't available, the CLI has equivalents:
`memex search <query>`, `memex read <slug>`, `memex list`.
"#;

pub fn run(claude_hook: bool) -> Result<()> {
    if claude_hook {
        // SessionStart hook envelope. Agent sees this at the start of its session.
        println!(
            "{{\"hookSpecificOutput\":{{\"hookEventName\":\"SessionStart\",\"additionalContext\":\"{}\"}}}}",
            json_escape(INSTRUCTIONS),
        );
    } else {
        print!("{INSTRUCTIONS}");
    }
    Ok(())
}
