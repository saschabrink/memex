# Changelog

All notable changes to memex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-04-17

Two doc-hygiene improvements: an annotated `memex.toml` example shipped
inside the binary, and `agent-instructions` now auto-generates this
project's source list from `memex.toml`. Together they remove two more
places where documentation could drift from configuration.

memex also sheds its residual MCP vocabulary. The tool is a CLI and that
choice is now explicit everywhere.

### Added
- `memex example-config` — prints a fully annotated `memex.toml` example
  embedded in the binary via `include_str!`. Covers every option plus
  copy-paste starting points for Elixir/Hex, Node `node_modules`, Ruby
  gems, Rust `cargo vendor`, Python `site-packages`, and Obsidian vaults.
  Works offline, matches the binary version exactly.
- `examples/memex.example.toml` — source of the embedded example.
- `memex agent-instructions` auto-generates a "This project's sources"
  section when a `memex.toml` is discoverable from the cwd. Source list
  stays drift-free because it is derived from the config, not transcribed
  into `CLAUDE.md` / `AGENTS.md`.

### Changed
- Agent-instructions brief replaces the fictional MCP tool-name block
  (`mcp__memex__*`) with a real CLI-command list (`search`, `read`,
  `list`, `write`, `edit`, `broken-refs`) plus a shell-plumbing example.
  Expresses that memex is a CLI and no MCP server is planned.
- `memex hook-advice --claude-hook` pre-write messages reference
  `memex read <id>` instead of the non-existent MCP tool.
- README: new "Works with Obsidian" section explaining vault
  compatibility; new "No MCP server" rationale; new "Roadmap" covering
  CI-on-PR gates, `memex doctor`, shell completions, `memex init`; new
  "The name" section on the Vannevar Bush origin.

### Removed
- All references to an MCP server / `mcp__memex__*` tools. memex is a
  CLI-only tool; shell-exec is the invocation surface.

## [0.5.0] - 2026-04-17

Doc-hygiene primitive: `broken-refs` now also scans external files for
`[[slug]]` references, catching drift in `CLAUDE.md`, `AGENTS.md`, and
similar agent-briefing files that live outside the blueprint sources.

### Added
- `also_scan = ["CLAUDE.md", ".claude/**/*.md", ...]` in `memex.toml` —
  top-level glob list (relative to `project_root`). Files matched are
  scanned for `[[slug]]` references during `broken-refs` and reported with
  their relative path as origin. They are NOT indexed as blueprints — no
  slugs produced, no search hits, no collisions. Tolerant of missing
  files: configuring `CLAUDE.md` when none exists is a no-op.
- Noise directories (`.git`, `node_modules`, `target`, `_build`, `.next`,
  `deps`) are skipped during `also_scan` walk, so a project-root glob like
  `**/*.md` stays fast and doesn't accidentally scan dep markdown.

### Rationale
Agent-brief files (`CLAUDE.md`, `AGENTS.md`, role docs) routinely reference
blueprints but aren't blueprints themselves. Without this, references drift
silently when blueprints are renamed or deleted. Adding them as a
`readonly` source would pollute `memex list` / `memex search` with
non-blueprint content; `also_scan` keeps them out of the KB while still
validating their links.

## [0.4.0] - 2026-04-17

Agent-onboarding primitives and per-dependency blueprints. Prebuilt Linux
binaries. `memex sync` is now safer by default (fast-forward only).

### Added
- `memex agent-instructions [--claude-hook]` — prints a generic,
  project-agnostic usage brief for LLM agents. With `--claude-hook`, emits a
  Claude Code `SessionStart` hook JSON envelope so the brief is delivered as
  `additionalContext` at session start. Does not require a `memex.toml`.
- Source option `index_filename = "<name>"` — files with this name represent
  their parent directory as a slug. `deps/ecto_context/usage-rules.md` becomes
  slug `deps/ecto_context`. Intended for indexing per-package usage rules
  shipped via Hex. Within-source slug collisions are detected at load time.
- Prebuilt release binary for `x86_64-unknown-linux-gnu`. Release workflow now
  runs as a matrix across macOS arm64 and Linux x86_64.
- `install.sh` auto-detects OS/arch and picks the matching artifact.

### Changed
- `memex sync` now uses `git pull --ff-only`. Divergent branches are reported
  as an error for that source but do not abort the overall sync. Readonly
  sources skip the push step entirely.
- Slug collision detection now catches conflicts within a single source (not
  just across sources). Necessary because `index_filename` can produce a slug
  that coincides with a normal `.md` slug in the same source.

## [0.3.0] - 2026-04-17

Hook infrastructure for deterministic, file-pattern-based context injection
during agent-driven editing. Plus read-only sources, ready for Hex-package
`usage-rules.md` integration.

### Added
- `hooks.toml` at project level (next to `memex.toml`) and optionally at each
  source mount root. Auto-discovered, merged in declaration order.
- Two hook primitives:
  - `[[pre-write]]` — inject blueprint references before a file is edited.
    Fields: `pattern` (regex, fancy-regex with backref support), `blueprint` or
    `blueprints` (string or list).
  - `[[post-write]]` — emit text advice after a file is written. Fields:
    `pattern`, `text` (with `${0..n}` capture-group substitution),
    optional `when_file_missing` / `when_file_exists` conditions.
- `memex hook-advice <file> --event pre-write|post-write [--claude-hook]` —
  print advice or emit the JSON expected by Claude Code's hook system.
- `readonly = true` on a source blocks `write`/`edit`/`delete`/`move` with an
  error and appends ` (read-only)` to titles in `list` / `search` output.
- Strong validation on `hooks.toml` load: `pre-write` with `text`, `post-write`
  with `blueprint`, both `blueprint` and `blueprints` set, missing required
  fields, or invalid regex all error with a clear message.
- `memex broken-refs` now also validates `blueprint` / `blueprints` references
  inside every loaded `hooks.toml`. Unresolved ids are reported with their
  event, pattern source, and originating source (or `project`).

### Changed
- First matching hook per event wins — deterministic, reihenfolge-controlled.
  Merge order: project-level first, then per-source in the order sources are
  declared in `memex.toml`, then entry order within each file.
- Regex engine is now `fancy-regex` (drop-in) to support backreferences,
  enabling patterns like `lib/platform/([^/]+)/\1\.ex$` for context-module
  detection.

## [0.2.0] - 2026-04-17

Breaking redesign of the configuration and storage model. No migration tool — blueprints are plain markdown files, copy them manually and rewrite `memex.toml`.

### Added
- `project_name` (mandatory) in `memex.toml`; used as the index directory name.
- Optional global `root` in `memex.toml` — project root relative to the config file.
- Per-source `mount` field: absolute slug-base, replaces the old `path` + `folders` combo.
- Per-source `prefix` field (default = source name; `""` allowed for at most one source to produce bare slugs).
- `include` / `exclude` glob lists on sources (e.g. `include = ["TODOS.md", "docs/**/*.md"]`), relative to `mount`.
- Automatic foreign-git detection: when enumerating a source, any subtree containing its own `.git/` is skipped (mirrors git submodule semantics).
- Push-on-write: `write` / `edit` / `delete` / `move` now auto-push for sources that have a `remote`. Failures warn but don't abort.
- Auto `.gitignore` entry for remote-backed sources that are cloned into the project tree.
- Tilde (`~/`) and environment-variable (`${VAR}`) expansion in `mount` paths.
- CHANGELOG.

### Changed
- Vector index moved from `<project>/.memex/vector_index.sqlite` to `~/Library/Caches/memex/indexes/<project_name>/vector_index.sqlite` (macOS) or `$XDG_CACHE_HOME/memex/indexes/<project_name>/` (Linux). The index is regenerable, so it belongs in the OS cache.
- Slug derivation: `<source.prefix>/<relpath-to-mount-without-.md>`. Case is preserved; slugs with slashes are first-class.
- `memex.toml` discovery is now strict — only `./memex.toml` in the current working directory. No upward search, no `config/`, no `.memex/`.
- Commits go to the git repo enclosing the file (project repo, or a cloned source's repo). No more per-source `git init`; memex does not create git repos, only clones when `remote` is set.
- `[[slug]]` link extractor accepts letters (any case), digits, `-`, `_`, `.`, `/`. Old lowercase-only rule replaced.
- `memex broken-refs` no longer enforces the project-vs-shared boundary rule (it was tied to the removed `project = true` field).

### Removed
- `path` field on sources (replaced by `mount`).
- `folders` field on sources (replaced by `include`/`exclude` globs).
- `project = true` field on sources.
- Config discovery under `config/` and `.memex/`.
- Default `.memex/<source>/` source path.

## [0.1.0] - 2026-04-16

Initial release.

### Added
- Semantic search over markdown "blueprints" using local `all-MiniLM-L6-v2` embeddings via fastembed-rs.
- Self-healing index: SHA-256 staleness check on every `search` / `list` re-embeds changed files automatically.
- Commands: `list`, `search`, `read`, `write`, `edit`, `delete`, `move`, `versions`, `diff`, `sync`, `rebuild-index`, `broken-refs`.
- Stdin support for `write` via `-` as the content argument.
- Single self-contained binary (bundled SQLite, statically linked ONNX Runtime).
- Prebuilt release binary for Apple Silicon macOS.
