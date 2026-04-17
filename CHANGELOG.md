# Changelog

All notable changes to memex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
