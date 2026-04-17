# memex

A personal knowledge base CLI. Store and search markdown notes ("blueprints") with semantic search powered by a local embedding model — no external services, no API keys, no network required after the initial model download.

Every write is automatically committed to git, so every blueprint has a full version history.

## Highlights

- **Blueprints are just markdown files in your project** — no hidden `.memex/` dump. Point memex at any folder structure you already have.
- **Single binary**, no runtime, no `node_modules`.
- **Fast cold start** (low tens of milliseconds).
- **Self-healing index**: every `search` / `list` runs a SHA-256 staleness check and re-embeds anything that changed on disk.
- **Pipe-friendly**: `echo "..." | memex write <id> -` reads content from stdin.
- **Auto push** on write for sources that have a remote; **read-only** sources block writes so shared blueprints can't be edited accidentally.
- **Hook-driven agent workflows**: file-pattern `hooks.toml` rules inject the right blueprints before an LLM edits a file. Works cleanly with Claude Code `PreToolUse` / `PostToolUse` hooks.
- **Self-documenting for agents**: `memex agent-instructions` emits a brief that can go straight into a SessionStart hook.

## Installation

```bash
curl -LsSf https://raw.githubusercontent.com/exfoundry/memex/main/install.sh | sh
```

Downloads the latest release binary to `~/.local/bin/memex`. Prebuilt binaries ship for:

- macOS Apple Silicon (`aarch64-apple-darwin`)
- Linux x86_64 (`x86_64-unknown-linux-gnu`)

Overrides: `MEMEX_INSTALL_DIR=/usr/local/bin` for a different target directory, `MEMEX_VERSION=v0.3.0` to pin a specific release.

Make sure `~/.local/bin` is on your `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Building from source

For other platforms, build from source:

```bash
git clone https://github.com/exfoundry/memex
cd memex
cargo build --release
install -m 0755 target/release/memex ~/.local/bin/memex
```

Requires Rust stable. If you use [Nix](https://nixos.org/), `nix develop` drops you into a shell with the right toolchain.

## Quick start

Create a `memex.toml` at your project root:

```toml
project_name = "myapp"

[myapp]
include = ["TODOS.md", "docs/**/*.md"]

[phoenix-liveview]
mount  = "docs/shared/phoenix-liveview"
remote = "git@github.com:exfoundry/phoenix-liveview.git"
```

Then:

```bash
memex sync                           # clones the remote-backed sources
memex write myapp/docs/vision "# Vision\n\nWhat we're building."
memex list                           # lists everything
memex search "database queries"      # semantic search
memex read myapp/docs/vision
```

## Configuration

### `memex.toml`

Must live at the directory you invoke `memex` from (no upward search, no `config/`, no `.memex/`). Explicit and deterministic.

```toml
project_name = "myapp"          # mandatory. identifies the index on disk.
root = "."                      # optional. project root relative to memex.toml.

[<source_name>]
mount   = "docs/something"      # optional. source dir relative to project root. Default: "."
prefix  = "custom"              # optional. slug prefix. Default: source_name. "" allowed (at most one).
include = ["**/*.md"]           # optional. glob patterns relative to mount. Default: ["**/*.md"].
exclude = ["drafts/**"]         # optional.
remote  = "git@..."             # optional. if mount doesn't exist, memex clones from here on `sync`.
readonly = true                 # optional. blocks writes, adds "(read-only)" to titles.
index_filename = "usage-rules.md"  # optional. files with this name represent their parent directory as a slug.
```

### Path resolution

- `root` is relative to the `memex.toml` directory. Absolute paths, `~/`, and `${ENV_VAR}` expansion are supported.
- `mount` per source is relative to `root`. Same expansion rules.
- `include` / `exclude` globs are relative to `mount`.

### Slug anatomy

Every blueprint has an **id** (slug). Rule:

```
slug = <source.prefix>/<path-relative-to-mount-without-.md>
```

If `prefix = ""`, the `<prefix>/` part is omitted.

Example layout:

```
~/Projects/myapp/
  memex.toml
  TODOS.md
  docs/
    vision.md
    tech/
      migrations.md
    shared/
      phoenix-liveview/         # cloned git repo (remote-backed source)
        context.md
```

With this config:

```toml
project_name = "myapp"

[myapp]
include = ["TODOS.md", "docs/**/*.md"]

[phoenix-liveview]
mount  = "docs/shared/phoenix-liveview"
remote = "git@github.com:exfoundry/phoenix-liveview.git"
```

You get these slugs:

| File | Slug |
|---|---|
| `TODOS.md` | `myapp/TODOS` |
| `docs/vision.md` | `myapp/docs/vision` |
| `docs/tech/migrations.md` | `myapp/docs/tech/migrations` |
| `docs/shared/phoenix-liveview/context.md` | `phoenix-liveview/context` |

The shared source gets short, portable slugs (`phoenix-liveview/context`) — the same file will have the same slug in any project that mounts it, regardless of where.

To drop the prefix for one source (e.g. a root-level notes source), set `prefix = ""`:

```toml
[notes]
include = ["TODOs.md", "SCRATCH.md"]
prefix  = ""
# slug for TODOs.md → "TODOs"
```

### Foreign git repos

Sources with a `remote` are cloned into their `mount` on `memex sync`. The mount directory gets added to the project's `.gitignore` automatically, so the clone is not tracked by the outer project.

When memex enumerates a source, it skips any subtree that contains its own `.git/` (e.g. other cloned sources). This mirrors git-submodule semantics.

### Index files

Set `index_filename` on a source to declare a filename that represents its parent directory. Like `index.html` on the web or `mod.rs` in Rust, the matching file becomes the slug of its containing folder:

```toml
[deps]
mount          = "deps"
include        = ["*/usage-rules.md"]
index_filename = "usage-rules.md"
readonly       = true
```

With this config, `deps/ecto_context/usage-rules.md` has slug `deps/ecto_context` (not `deps/ecto_context/usage-rules`). The primary use case is indexing per-package usage rules that ship with Hex dependencies — the slug points directly at the package name.

Non-matching files in the same source fall through to the normal slug rule. If two files in one source would collide on the same slug (e.g. both `deps/foo.md` and `deps/foo/usage-rules.md` exist), the config fails to load.

## Commands

```
memex <command> [args...] [--config <path>]
```

### Reading

| Command | Description |
|---|---|
| `memex list` | List all blueprints across all sources. |
| `memex read <id>` | Print full content (prepends `# <title>` if the blueprint's content doesn't start with one). |
| `memex search <query> [--limit N]` | Semantic search. Default limit 5. Output: `<id>  [distance]  <title>`. |

Both `list` and `search` run a SHA-256 staleness check first. Files that changed on disk are re-embedded automatically. Files that disappeared are removed from the index.

### Writing

| Command | Description |
|---|---|
| `memex write <id> <content>` | Create or overwrite. Commits to the file's enclosing git repo. Pushes if the source has a `remote`. `<content>` as `-` reads from stdin. |
| `memex edit <id> <old> <new>` | Literal find-and-replace (first occurrence). Commits and re-embeds. |
| `memex delete <id>` | Delete file + remove from index. Commits. |
| `memex move <old_id> <new_id>` | Rename or move between sources. Uses `git mv` inside a source; cross-source moves are write-then-delete with two commits. |

Every write op embeds the new content synchronously, so the index is always consistent after the command returns.

Because `<content>` is a shell argument, escape sequences like `\n` are passed literally. Use stdin for multi-line content:

```bash
memex write myapp/docs/foo - <<'EOF'
# Foo

Multi-line
content.
EOF
```

### History

| Command | Description |
|---|---|
| `memex versions <id>` | Git log for the blueprint. Output: `<hash>  <date>  <message>`. |
| `memex diff <id> <hash>` | `git show <hash> -- <file>`. |

### Maintenance

| Command | Description |
|---|---|
| `memex sync` | For each source with a `remote`: clone if missing, else `git pull --ff-only` (+ `git push` for writable sources). Never merges or rebases. |
| `memex rebuild-index` | Drop the index and rebuild from disk. Only needed after schema changes or index corruption — normal stale-check handles everything else. |
| `memex broken-refs` | Find `[[slug]]` references that don't resolve — in blueprints, `hooks.toml`, and any external files listed under `also_scan`. |
| `memex hook-advice <file> --event pre-write\|post-write [--claude-hook]` | Look up matching hooks for `<file>`. See [Hooks](#hooks). |
| `memex agent-instructions [--claude-hook]` | Print generic usage instructions for LLM agents. Does not require a `memex.toml`. See [Agent onboarding](#agent-onboarding). |

### Cross-references

Inside blueprint content, use `[[slug]]` to reference another blueprint. `broken-refs` considers a reference resolved if either:

- **Full slug match** — e.g. `[[shared/phoenix-liveview/context]]` matches exactly that slug.
- **Bare-name match** — a ref without slashes (e.g. `[[context]]`) matches any slug whose last path segment equals the ref. Convenient for short links inside a tightly-coupled set of blueprints.

If multiple slugs share the same last segment, a bare ref is still considered resolved (ambiguity is not flagged). Use the full slug when you need to disambiguate.

### Checking external files (`also_scan`)

Agent-brief files like `CLAUDE.md`, `AGENTS.md`, or per-tool docs under `.claude/` typically reference blueprints but aren't blueprints themselves. Add them to `also_scan` in `memex.toml` (top-level, not per-source) and `broken-refs` will include them without indexing them:

```toml
project_name = "myproject"
also_scan = ["CLAUDE.md", "AGENTS.md", ".claude/**/*.md"]

[shared]
mount  = "docs/shared"
remote = "..."
```

Paths are globs relative to `project_root`. Matched files are scanned for `[[slug]]` references; no slugs are produced, no collisions, no search hits. Missing files are silently ignored, so configuring `CLAUDE.md` on a project without one is a no-op. Noise directories (`.git`, `node_modules`, `target`, `_build`, `.next`, `deps`) are skipped during the walk.

## Agent onboarding

memex ships a built-in instruction brief for LLM agents. Rather than copy tool-usage boilerplate into every project's `CLAUDE.md` / `AGENTS.md`, the brief is emitted by the CLI and updates automatically with memex upgrades.

### Session-start hook (recommended)

If you use Claude Code, add a `SessionStart` hook so the brief is delivered as context automatically. In `.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      { "command": "memex agent-instructions --claude-hook" }
    ]
  }
}
```

Each session, the agent sees the current usage brief as `additionalContext`. No round-trip, no manual copying.

### CLAUDE.md / AGENTS.md snippet

If you prefer to keep your agent config file self-contained, or target tools that don't support SessionStart hooks, add this single paragraph to your project's agent file:

```markdown
## memex

Before using memex tools for the first time in a session, run:

    memex agent-instructions

If the command isn't found, install it first:

    curl -LsSf https://raw.githubusercontent.com/exfoundry/memex/main/install.sh | sh
```

The CLI output is plain markdown, safe to inline into any agent's context.

### What the brief covers

- MCP tool names and when to use each (`search_blueprints`, `read_blueprint`, `list_blueprints`).
- When to reach for memex vs. asking the user vs. guessing.
- What not to do (paraphrasing blueprint content, skipping hook advice, writing to read-only sources).
- CLI fallbacks for environments without MCP.

The brief is project-agnostic. Project-specific context (architecture, conventions, no-gos) belongs in your `CLAUDE.md` / `AGENTS.md` alongside — not duplicated into memex.

## Hooks

memex can answer "what blueprints or advice apply to this file?" for agent-driven editing workflows (e.g. Claude Code `PreToolUse` / `PostToolUse` hooks). Hooks live in `hooks.toml` files and are dispatched by file-path regex.

### Discovery

- `<project_root>/hooks.toml` — project-level, loaded first.
- `<source_mount>/hooks.toml` — per-source, loaded in the order sources appear in `memex.toml`.

Within each file, entries are evaluated in declaration order. **First match per event wins.**

### Primitives

Two event tables, both as TOML array-of-tables:

```toml
# Before writing a file: tell the agent which blueprints to read first.

[[pre-write]]
pattern   = "_live\\.ex$"
blueprint = "phoenix-liveview/liveview"            # string

[[pre-write]]
pattern    = "lib/platform/([^/]+)/\\1\\.ex$"      # backref: context/context.ex
blueprints = ["phoenix-liveview/context",          # list — "blueprint" also accepts a list
              "phoenix-liveview/context-testing"]

# After writing: emit text advice, optionally conditional on another file's presence.

[[post-write]]
pattern           = "^lib/(.+)\\.ex$"
text              = "No test file found. Expected: test/${1}_test.exs — write the test before moving on."
when_file_missing = "test/${1}_test.exs"           # only fires when this path doesn't exist
```

- **Regex:** fancy-regex syntax (PCRE-ish, with backreference support).
- **Substitution:** `${0}` is the whole match; `${1}`, `${2}`, … are capture groups. Works in `text`, `when_file_missing`, `when_file_exists`.
- **Conditions:** `when_file_missing` / `when_file_exists` — hook only fires when the named path (relative to project root) is absent / present.
- **Blueprint keys:** `blueprint` and `blueprints` are interchangeable, both accept a string or a list. Setting both at once is an error.
- **Validation:** `pre-write` with `text`, `post-write` with `blueprint`, missing required fields, or invalid regex all error at load time with a clear message.

### Use with Claude Code

Project-level hook shell-script becomes a three-liner, independent of which patterns are configured:

```bash
#!/usr/bin/env bash
FILE_PATH=$(jq -r '.tool_input.file_path // ""')
memex hook-advice "$FILE_PATH" --event pre-write --claude-hook
```

`memex hook-advice` without `--claude-hook` prints a human-readable line for debugging. No match → nothing is printed (which Claude Code treats as a no-op).

## How it works

### Embeddings

Memex embeds blueprint content using [`all-MiniLM-L6-v2`](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) (384-dim, ONNX, ~87MB). The model runs locally via [fastembed-rs](https://github.com/Anush008/fastembed-rs), which wraps [ONNX Runtime](https://onnxruntime.ai/). Mean pooling and L2 normalization are applied automatically — similarity is cosine.

Search is a full table scan with in-memory cosine computation. At the scale memex targets (hundreds to low thousands of blueprints), this is well under 50ms per query.

### Storage layout

- **Blueprint files**: live in your project, wherever you point `mount` at. Committed to whichever git repo encloses them (your project repo, or a cloned remote source).
- **Vector index**: `~/Library/Caches/memex/indexes/<project_name>/vector_index.sqlite` on macOS, `$XDG_CACHE_HOME/memex/indexes/<project_name>/` on Linux. Regenerable from source files with `memex rebuild-index`.
- **Model cache** (downloaded once, shared across projects):
  - macOS: `~/Library/Caches/memex/models/`
  - Linux: `$XDG_CACHE_HOME/memex/models/` (defaults to `~/.cache/memex/models/`)

The index lives in the OS cache directory because it's cheap to rebuild. If you delete a project, its index will eventually get cleaned up by the OS.

### Stale-index check

On every `search` and `list`:

1. Enumerate all blueprint files on disk (per source's globs, skipping foreign git subtrees).
2. Compute SHA-256 of each file's bytes.
3. Compare against `content_hash` stored in the index.
4. Re-embed files whose hash changed (batched in one ONNX call). Delete index entries whose file disappeared.

In the steady state, this is ~10ms for a few dozen blueprints — fast enough to run unconditionally.

## Development

Requires [Nix](https://nixos.org/) with flakes, or a system-wide Rust toolchain.

```bash
nix develop                  # enters shell with rustc, cargo, cmake, pkg-config
cargo build                  # debug build
cargo build --release        # release build
cargo test                   # fast tests
cargo test -- --ignored      # tests that exercise the embedder (~87MB model download on first run)
```

Run against an existing project:

```bash
cd path/to/project-with-memex.toml
/path/to/memex/target/release/memex list
```

The binary is fully self-contained: SQLite is bundled via `rusqlite`'s `bundled` feature, ONNX Runtime is statically linked by the `ort` dependency, libgit2 is not used (memex shells out to `git`, which is assumed present).

## Releases

Releases are tagged `vX.Y.Z` and built automatically by [`.github/workflows/release.yml`](.github/workflows/release.yml) on GitHub-hosted runners. The workflow builds a matrix:

- `aarch64-apple-darwin` on `macos-14`
- `x86_64-unknown-linux-gnu` on `ubuntu-22.04`

Each target produces `memex-<target>.tar.gz` + SHA-256 sidecar, both attached to the GitHub Release.

To cut a release:

```bash
# bump version in Cargo.toml, commit
git tag v0.4.0
git push --tags
```

## License

MIT
