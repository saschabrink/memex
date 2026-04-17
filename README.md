# memex

A personal knowledge base CLI. Store and search markdown notes ("blueprints") with semantic search powered by a local embedding model — no external services, no API keys, no network required after the initial model download.

Every write is automatically committed to git, so every blueprint has a full version history.

## Highlights

- **Blueprints are just markdown files in your project** — no hidden `.memex/` dump. Point memex at any folder structure you already have.
- **Single binary**, no runtime, no `node_modules`.
- **Fast cold start** (low tens of milliseconds).
- **Self-healing index**: every `search` / `list` runs a SHA-256 staleness check and re-embeds anything that changed on disk.
- **Pipe-friendly**: `echo "..." | memex write <id> -` reads content from stdin.
- **Auto push** on write for sources that have a remote — no manual sync to keep shared blueprints up to date.

## Installation

### Apple Silicon (macOS arm64)

```bash
curl -sSL https://raw.githubusercontent.com/exfoundry/memex/main/install.sh | bash
```

This downloads the latest release binary to `~/.local/bin/memex`. Override the location with `MEMEX_INSTALL_DIR=/usr/local/bin`. Pin a specific version with `MEMEX_VERSION=v0.2.0`.

Make sure `~/.local/bin` is on your `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Linux / Intel Mac / building from source

Prebuilt binaries ship only for Apple Silicon at the moment. Everywhere else, build from source — `cargo build --release` produces a self-contained `memex` binary.

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
| `memex sync` | For each source with a `remote`: clone if missing, else `git pull` + `git push`. |
| `memex rebuild-index` | Drop the index and rebuild from disk. Only needed after schema changes or index corruption — normal stale-check handles everything else. |
| `memex broken-refs` | Find `[[slug]]` references in blueprint content that don't resolve to an existing blueprint. |

### Cross-references

Inside blueprint content, use `[[slug]]` to reference another blueprint. `broken-refs` considers a reference resolved if either:

- **Full slug match** — e.g. `[[shared/phoenix-liveview/context]]` matches exactly that slug.
- **Bare-name match** — a ref without slashes (e.g. `[[context]]`) matches any slug whose last path segment equals the ref. Convenient for short links inside a tightly-coupled set of blueprints.

If multiple slugs share the same last segment, a bare ref is still considered resolved (ambiguity is not flagged). Use the full slug when you need to disambiguate.

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

Releases are tagged `vX.Y.Z` and built automatically by [`.github/workflows/release.yml`](.github/workflows/release.yml) on GitHub-hosted `macos-14` runners (Apple Silicon). The workflow packages `target/aarch64-apple-darwin/release/memex` as `memex-aarch64-apple-darwin.tar.gz` plus a SHA-256 sidecar and attaches both to the GitHub Release.

To cut a release:

```bash
# bump version in Cargo.toml, commit
git tag v0.2.0
git push --tags
```

## License

MIT
