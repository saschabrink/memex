# memex

A personal knowledge base CLI. Store and search markdown notes ("blueprints") with semantic search powered by a local embedding model — no external services, no API keys, no network required after the initial model download.

Every write is automatically committed to git, so every blueprint has a full version history.

## Highlights

- **Single binary**, no runtime, no `node_modules`.
- **Fast cold start** (low tens of milliseconds).
- **Self-healing index**: every `search` / `list` runs a SHA-256 staleness check and re-embeds any file that changed on disk (e.g. after `git pull` in a shared source).
- **Pipe-friendly**: `echo "..." | memex write <id> -` reads content from stdin.

## Installation

### Apple Silicon (macOS arm64)

```bash
curl -sSL https://raw.githubusercontent.com/exfoundry/memex/main/install.sh | bash
```

This downloads the latest release binary to `~/.local/bin/memex`. Override the location with `MEMEX_INSTALL_DIR=/usr/local/bin`. Pin a specific version with `MEMEX_VERSION=v0.1.0`.

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

### Updating

Re-run the install script:

```bash
curl -sSL https://raw.githubusercontent.com/exfoundry/memex/main/install.sh | bash
```

It always fetches the latest release and overwrites the existing binary.

## Quick start

Create a `memex.toml` in your project (memex searches `./`, `./.memex/`, `./memex/`, and `./config/` for it):

```toml
[myapp]
project = true
folders = ["notes"]

[shared]
remote = "https://github.com/your-org/shared-blueprints.git"
```

Then:

```bash
memex sync                           # clones the shared remote (first run only)
memex write myapp/notes/ecto "# Ecto\n\nDatabase access patterns."
memex list                           # list everything (triggers auto-reindex if anything changed)
memex search "database queries"      # semantic search
memex read myapp/notes/ecto
```

## Configuration

### `memex.toml`

Each top-level TOML section declares a blueprint **source** — a directory of markdown files with its own git history.

```toml
[myapp]
project = true                       # this source is project-specific (not shared)
path = "./blueprints"                # optional; defaults to .memex/<name>
folders = ["tech", "notes"]          # subfolders to index; ["."] = whole source

[shared]
remote = "https://github.com/org/shared-blueprints.git"
                                     # cloned on `memex sync` to .memex/shared
```

Fields:

| Field | Required | Description |
|---|---|---|
| `path` | no | Directory holding the markdown. Defaults to `.memex/<name>`. Supports `~/`. |
| `remote` | no | Git remote. If the source directory doesn't exist, `memex sync` clones from here. |
| `folders` | no | Subfolders (relative to `path`) to index. Defaults to `["."]` (the entire source). |
| `project` | no | `true` marks this source as project-specific. Affects `broken-refs` (a shared source cannot reference blueprints that only exist in project-specific sources). |

### Blueprint IDs

An ID is `<source_name>/<path-without-.md>`, e.g. `myapp/notes/ecto` refers to `.memex/myapp/notes/ecto.md`. IDs are the currency for every command — they're stable, they're what you paste into LLM prompts, and they double as the filesystem path.

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

Both `list` and `search` run a SHA-256 staleness check first. Files that changed on disk since the last index write are re-embedded automatically. Files that disappeared are removed from the index. Files that didn't change cost a single hash comparison (~0.5ms each).

### Writing

| Command | Description |
|---|---|
| `memex write <id> <content>` | Create or overwrite. Commits to the source's git repo. `<content>` as `-` reads from stdin. |
| `memex edit <id> <old> <new>` | Literal find-and-replace (first occurrence). Commits and re-embeds. |
| `memex delete <id>` | Delete file + remove from index. Commits. |
| `memex move <old_id> <new_id>` | Rename or move between sources. Uses `git mv` inside a source; cross-source moves are write-then-delete with two commits. |

Every write op embeds the new content synchronously, so the index is always consistent after the command returns.

Because `<content>` is a shell argument, escape sequences like `\n` are passed literally. Use stdin for multi-line content:

```bash
memex write myapp/notes/foo - <<'EOF'
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
| `memex broken-refs` | Find `[[slug]]` references in blueprint content that don't resolve to an existing blueprint. Shared sources may not reference project-specific blueprints. |

## How it works

### Embeddings

Memex embeds blueprint content using [`all-MiniLM-L6-v2`](https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2) (384-dim, ONNX, ~87MB). The model runs locally via [fastembed-rs](https://github.com/Anush008/fastembed-rs), which wraps [ONNX Runtime](https://onnxruntime.ai/). Mean pooling and L2 normalization are applied automatically — similarity is cosine.

Search is a full table scan with in-memory cosine computation. At the scale memex targets (hundreds to low thousands of blueprints), this is well under 50ms per query. If you ever cross into tens of thousands, the right answer is to plug in `sqlite-vec` — until then it's premature.

### Storage layout

Per-project:

```
<projectDir>/
  config/memex.toml               # or memex.toml, .memex/memex.toml
  .memex/
    vector_index.sqlite           # blueprints + embeddings (schema v2)
    <source_name>/                # each source is a standalone git repo
      .git/
      .gitignore                  # ignores .db/
      notes/*.md
```

The SQLite index lives under the project. Blueprint files and their git history live in the source directories, which can be shared across projects (e.g. point two projects at the same `shared/` clone and they both index it).

### Model cache (downloaded once, shared across projects)

- **macOS**: `~/Library/Caches/memex/models/`
- **Linux**: `$XDG_CACHE_HOME/memex/models/` (defaults to `~/.cache/memex/models/`)

### Stale-index check

On every `search` and `list`:

1. Enumerate all blueprint files on disk (per source's configured folders).
2. Compute SHA-256 of each file's bytes.
3. Compare against `content_hash` stored in the index.
4. Re-embed files whose hash changed (batched in one ONNX call). Delete index entries whose file disappeared.

In the steady state (nothing changed), this is ~10ms for a few dozen blueprints — fast enough to run unconditionally. When files *have* changed, only those specific files are re-embedded.

This is what fixes the "I pulled the shared repo but search still returns old results" failure mode.

## Development

Requires [Nix](https://nixos.org/) with flakes, or a system-wide Rust toolchain.

```bash
nix develop                  # enters shell with rustc, cargo, cmake, pkg-config
cargo build                  # debug build
cargo build --release        # release build (~20s, 28MB binary)
cargo check                  # fast type-check without codegen
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
