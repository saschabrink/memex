use anyhow::{anyhow, bail, Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    /// Absolute path where this source lives in the filesystem.
    pub mount: PathBuf,
    /// Slug prefix. Default is the source name; may be empty (at most one source).
    pub prefix: String,
    pub remote: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

impl Source {
    fn include_set(&self) -> Result<GlobSet> {
        build_globset(&self.include)
    }
    fn exclude_set(&self) -> Result<GlobSet> {
        build_globset(&self.exclude)
    }
}

#[derive(Debug)]
pub struct MemexConfig {
    pub project_name: String,
    /// Directory that contains `memex.toml`.
    pub config_dir: PathBuf,
    /// Resolved project root (= config_dir / root).
    pub project_root: PathBuf,
    pub config_path: PathBuf,
    pub sources: Vec<Source>,
}

#[derive(Debug)]
pub struct ResolvedBlueprint<'a> {
    pub source: &'a Source,
    pub file_path: PathBuf,
}

impl MemexConfig {
    /// `<cache_dir>/memex/indexes/<project_name>/vector_index.sqlite`
    pub fn db_path(&self) -> PathBuf {
        let base = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("memex")
            .join("indexes")
            .join(&self.project_name)
            .join("vector_index.sqlite")
    }

    /// Compute slug for a file. `file_path` must be under `source.mount`.
    pub fn blueprint_id(&self, source: &Source, file_path: &Path) -> String {
        let rel = file_path
            .strip_prefix(&source.mount)
            .unwrap_or(file_path)
            .to_string_lossy()
            .replace('\\', "/");
        let stripped = rel.strip_suffix(".md").unwrap_or(&rel);
        if source.prefix.is_empty() {
            stripped.to_string()
        } else {
            format!("{}/{}", source.prefix, stripped)
        }
    }

    /// Reverse: turn a slug into a concrete source + file path.
    /// Longest non-empty prefix wins; empty-prefix source is the fallback.
    pub fn resolve_blueprint(&self, id: &str) -> Result<ResolvedBlueprint<'_>> {
        let mut best: Option<&Source> = None;
        let mut best_len = 0usize;
        let mut fallback: Option<&Source> = None;

        for source in &self.sources {
            if source.prefix.is_empty() {
                fallback = Some(source);
                continue;
            }
            let pfx = format!("{}/", source.prefix);
            if id.starts_with(&pfx) && source.prefix.len() > best_len {
                best = Some(source);
                best_len = source.prefix.len();
            }
        }

        let (source, rest) = if let Some(s) = best {
            let rest = &id[s.prefix.len() + 1..];
            (s, rest)
        } else if let Some(s) = fallback {
            (s, id)
        } else {
            return Err(anyhow!(
                "No source matches slug '{id}'. Configured sources: {}",
                self.sources
                    .iter()
                    .map(|s| {
                        if s.prefix.is_empty() {
                            format!("{} (prefix=\"\")", s.name)
                        } else {
                            format!("{} (prefix={})", s.name, s.prefix)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        };

        let file_path = source.mount.join(format!("{rest}.md"));
        Ok(ResolvedBlueprint { source, file_path })
    }

    /// Enumerate all blueprint files across all sources.
    /// Skips nested `.git` subtrees (foreign clones).
    pub fn all_blueprints(&self) -> Vec<(&Source, PathBuf)> {
        let mut out = Vec::new();
        for source in &self.sources {
            match enumerate_source(source) {
                Ok(files) => {
                    for f in files {
                        out.push((source, f));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "warning: enumerating source {}: {}",
                        source.name, e
                    );
                }
            }
        }
        out
    }

    pub fn all_source_names(&self) -> Vec<String> {
        self.sources.iter().map(|s| s.name.clone()).collect()
    }

    pub fn extract_title(&self, content: &str) -> String {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("# ") {
                return rest.trim().to_string();
            }
        }
        "Untitled".to_string()
    }

    /// Idempotent setup: clone missing remote sources, create caches dir,
    /// add clones to the project's .gitignore, and verify no slug collisions.
    pub fn ensure_initialized(&self) -> Result<()> {
        if let Some(parent) = self.db_path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        for source in &self.sources {
            if !source.mount.exists() {
                if let Some(remote) = &source.remote {
                    if let Some(p) = source.mount.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    crate::git::clone(remote, &source.mount)?;
                    self.add_to_project_gitignore(&source.mount)?;
                }
                // else: nothing — enumerate will just yield no files
            } else if source.remote.is_some() {
                // existing path for a remote source — make sure it's in .gitignore
                self.add_to_project_gitignore(&source.mount)?;
            }
        }

        self.check_collisions()?;
        Ok(())
    }

    fn add_to_project_gitignore(&self, source_mount: &Path) -> Result<()> {
        // Only if project_root itself is a git repo.
        if !self.project_root.join(".git").exists() {
            return Ok(());
        }
        let rel = match source_mount.strip_prefix(&self.project_root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => return Ok(()), // mount is outside project root, nothing to ignore
        };
        let line = format!("/{rel}/");
        let gi = self.project_root.join(".gitignore");
        let current = std::fs::read_to_string(&gi).unwrap_or_default();
        if current.lines().any(|l| l.trim() == line.trim()) {
            return Ok(());
        }
        let mut new = current;
        if !new.is_empty() && !new.ends_with('\n') {
            new.push('\n');
        }
        new.push_str(&line);
        new.push('\n');
        std::fs::write(&gi, new).with_context(|| format!("writing {}", gi.display()))?;
        Ok(())
    }

    fn check_collisions(&self) -> Result<()> {
        let mut seen: BTreeMap<String, String> = BTreeMap::new();
        for (source, file_path) in self.all_blueprints() {
            let id = self.blueprint_id(source, &file_path);
            if let Some(other) = seen.insert(id.clone(), source.name.clone()) {
                if other != source.name {
                    bail!(
                        "slug collision: '{id}' is produced by sources '{other}' and '{}'",
                        source.name
                    );
                }
            }
        }
        Ok(())
    }
}

// ---------- parsing ----------

#[derive(Deserialize, Debug)]
struct RawConfig {
    project_name: String,
    #[serde(default = "default_root")]
    root: String,
    #[serde(flatten)]
    sources: BTreeMap<String, RawSource>,
}

fn default_root() -> String {
    ".".to_string()
}

#[derive(Deserialize, Debug)]
struct RawSource {
    mount: Option<String>,
    prefix: Option<String>,
    remote: Option<String>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
}

/// Only looks for `./memex.toml` in the given directory. No upward search,
/// no `config/`, no `.memex/`. Explicit.
pub fn discover(start_dir: &Path) -> Result<PathBuf> {
    let candidate = start_dir.join("memex.toml");
    if candidate.exists() {
        return Ok(candidate);
    }
    Err(anyhow!(
        "No memex.toml found in {}. Create a memex.toml to configure your blueprint sources.",
        start_dir.display()
    ))
}

pub fn load(start_dir: &Path, override_path: Option<&Path>) -> Result<MemexConfig> {
    let config_path = match override_path {
        Some(p) => p.to_path_buf(),
        None => discover(start_dir)?,
    };
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let raw: RawConfig =
        toml::from_str(&content).with_context(|| format!("parsing {}", config_path.display()))?;

    if raw.project_name.trim().is_empty() {
        bail!("'project_name' in memex.toml must be non-empty");
    }

    let config_dir = config_path
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent"))?
        .to_path_buf();
    let project_root = resolve_path(&config_dir, &raw.root);

    let mut sources = Vec::new();
    let mut empty_prefix_count = 0;
    for (name, raw_src) in raw.sources {
        let mount_rel = raw_src.mount.as_deref().unwrap_or(".");
        let mount = resolve_path(&project_root, mount_rel);
        let prefix = raw_src.prefix.unwrap_or_else(|| name.clone());
        if prefix.is_empty() {
            empty_prefix_count += 1;
        }
        let include = raw_src
            .include
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| vec!["**/*.md".to_string()]);
        let exclude = raw_src.exclude.unwrap_or_default();

        sources.push(Source {
            name,
            mount,
            prefix,
            remote: raw_src.remote,
            include,
            exclude,
        });
    }

    if empty_prefix_count > 1 {
        bail!(
            "At most one source may set prefix = \"\" (found {})",
            empty_prefix_count
        );
    }

    Ok(MemexConfig {
        project_name: raw.project_name,
        config_dir,
        project_root,
        config_path,
        sources,
    })
}

fn resolve_path(base: &Path, p: &str) -> PathBuf {
    let expanded = expand_home_and_env(p);
    if expanded.is_absolute() {
        expanded
    } else {
        base.join(expanded)
    }
}

fn expand_home_and_env(p: &str) -> PathBuf {
    // ${VAR} expansion
    let mut out = String::with_capacity(p.len());
    let bytes = p.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'$' && bytes[i + 1] == b'{' {
            if let Some(end) = p[i + 2..].find('}') {
                let var = &p[i + 2..i + 2 + end];
                let val = std::env::var(var).unwrap_or_default();
                out.push_str(&val);
                i += 2 + end + 1;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }

    if let Some(rest) = out.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if out == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(out)
}

fn build_globset(patterns: &[String]) -> Result<GlobSet> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let glob = Glob::new(p).with_context(|| format!("invalid glob: {p}"))?;
        b.add(glob);
    }
    b.build().context("building globset")
}

/// Walk a source's mount, applying include/exclude globs and skipping
/// foreign git subtrees.
fn enumerate_source(source: &Source) -> Result<Vec<PathBuf>> {
    if !source.mount.exists() {
        return Ok(Vec::new());
    }
    let include = source.include_set()?;
    let exclude = source.exclude_set()?;
    let mount_has_git = source.mount.join(".git").exists();

    let mut out = Vec::new();
    let walker = walkdir::WalkDir::new(&source.mount)
        .into_iter()
        .filter_entry(|e| {
            let p = e.path();
            // Skip the mount itself from skip-logic
            if p == source.mount {
                return true;
            }
            if e.file_type().is_dir() {
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    return true;
                };
                // Always skip .git directories
                if name == ".git" {
                    return false;
                }
                // Skip common noise
                if matches!(name, "node_modules" | "target" | "_build" | ".next") {
                    return false;
                }
                // Skip foreign git repos (has .git/ but isn't the mount itself)
                if p.join(".git").exists() && !(mount_has_git && p == source.mount) {
                    return false;
                }
            }
            true
        });

    for entry in walker {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let rel = match path.strip_prefix(&source.mount) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if !include.is_match(rel) {
            continue;
        }
        if !exclude.is_empty() && exclude.is_match(rel) {
            continue;
        }
        out.push(path.to_path_buf());
    }
    out.sort();
    Ok(out)
}

/// Walk up from `start` until a directory containing `.git/` is found.
/// Returns the repo root, or None.
pub fn find_enclosing_repo(start: &Path) -> Option<PathBuf> {
    let mut cur = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        cur = cur.parent()?.to_path_buf();
    }
}
