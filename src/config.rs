use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Source {
    pub name: String,
    pub path: PathBuf,
    pub remote: Option<String>,
    pub folders: Vec<String>,
    pub project: bool,
}

#[derive(Debug)]
pub struct MemexConfig {
    pub project_dir: PathBuf,
    pub config_path: PathBuf,
    pub sources: Vec<Source>,
}

#[derive(Debug)]
pub struct ResolvedBlueprint<'a> {
    pub source: &'a Source,
    pub file_path: PathBuf,
    pub folder: String,
}

impl MemexConfig {
    pub fn db_path(&self) -> PathBuf {
        self.project_dir.join(".memex").join("vector_index.sqlite")
    }

    pub fn all_folders(&self) -> Vec<String> {
        let mut out = Vec::new();
        for s in &self.sources {
            for f in &s.folders {
                if f == "." {
                    out.push(s.name.clone());
                } else {
                    out.push(format!("{}/{}", s.name, f));
                }
            }
        }
        out
    }

    pub fn resolve_blueprint(&self, id: &str) -> Result<ResolvedBlueprint<'_>> {
        for source in &self.sources {
            let prefix = format!("{}/", source.name);
            if let Some(rel) = id.strip_prefix(&prefix) {
                let file_path = source.path.join(format!("{rel}.md"));
                let folder = self.blueprint_folder(source, &file_path);
                return Ok(ResolvedBlueprint {
                    source,
                    file_path,
                    folder,
                });
            }
        }
        Err(anyhow!(
            "No source found for blueprint '{id}'. Available sources: {}",
            self.sources
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    pub fn blueprint_id(&self, source: &Source, file_path: &Path) -> String {
        let rel = file_path.strip_prefix(&source.path).unwrap_or(file_path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let without_ext = rel_str.strip_suffix(".md").unwrap_or(&rel_str);
        format!("{}/{}", source.name, without_ext)
    }

    pub fn blueprint_folder(&self, source: &Source, file_path: &Path) -> String {
        let rel = file_path.strip_prefix(&source.path).unwrap_or(file_path);
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        for folder in &source.folders {
            if folder == "." {
                return source.name.clone();
            }
            if rel_str.starts_with(&format!("{folder}/")) || rel_str == *folder {
                return format!("{}/{}", source.name, folder);
            }
        }
        let dir = Path::new(&rel_str)
            .parent()
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        format!("{}/{}", source.name, dir)
    }

    pub fn all_blueprints(&self) -> Vec<(&Source, PathBuf)> {
        let mut out = Vec::new();
        for source in &self.sources {
            for folder in &source.folders {
                let folder_path = if folder == "." {
                    source.path.clone()
                } else {
                    source.path.join(folder)
                };
                if folder_path.exists() {
                    let mut files = glob_md(&folder_path);
                    files.sort();
                    for fp in files {
                        out.push((source, fp));
                    }
                }
            }
        }
        out
    }

    pub fn ensure_initialized(&self) -> Result<()> {
        if let Some(parent) = self.db_path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        for source in &self.sources {
            let git_dir = source.path.join(".git");
            if !source.path.exists() {
                if let Some(remote) = &source.remote {
                    crate::git::clone(remote, &source.path)?;
                } else {
                    std::fs::create_dir_all(&source.path)?;
                    crate::git::init_with_gitignore(&source.path, &source.name)?;
                }
            } else if !git_dir.exists() {
                if !crate::git::is_inside_repo(&source.path)? {
                    crate::git::init_with_gitignore(&source.path, &source.name)?;
                }
            }
            for folder in &source.folders {
                if folder != "." {
                    std::fs::create_dir_all(source.path.join(folder))?;
                }
            }
        }
        Ok(())
    }

    pub fn extract_title(&self, content: &str) -> String {
        for line in content.lines() {
            if let Some(rest) = line.strip_prefix("# ") {
                return rest.trim().to_string();
            }
        }
        "Untitled".to_string()
    }
}

#[derive(Deserialize, Debug)]
struct RawSource {
    path: Option<String>,
    remote: Option<String>,
    folders: Option<Vec<String>>,
    #[serde(default)]
    project: bool,
}

pub fn discover(start_dir: &Path) -> Result<PathBuf> {
    let candidates = [
        start_dir.join("memex.toml"),
        start_dir.join(".memex").join("memex.toml"),
        start_dir.join("memex").join("memex.toml"),
        start_dir.join("config").join("memex.toml"),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Err(anyhow!(
        "No memex.toml found. Looked in:\n  {}\n\nCreate a memex.toml to configure your blueprint sources.",
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n  ")
    ))
}

pub fn load(start_dir: &Path, override_path: Option<&Path>) -> Result<MemexConfig> {
    let config_path = match override_path {
        Some(p) => p.to_path_buf(),
        None => discover(start_dir)?,
    };
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    let raw: BTreeMap<String, RawSource> =
        toml::from_str(&content).with_context(|| format!("parsing {}", config_path.display()))?;

    let parent = config_path
        .parent()
        .ok_or_else(|| anyhow!("config path has no parent"))?
        .to_path_buf();
    let project_dir = match parent.file_name().and_then(|n| n.to_str()) {
        Some("config") | Some(".memex") => parent
            .parent()
            .ok_or_else(|| anyhow!("config dir has no parent"))?
            .to_path_buf(),
        _ => parent,
    };

    let mut sources = Vec::new();
    for (name, raw) in raw {
        let resolved_path = match raw.path {
            Some(p) => project_dir.join(expand_home(&p)),
            None => project_dir.join(".memex").join(&name),
        };
        let folders = raw
            .folders
            .filter(|f| !f.is_empty())
            .unwrap_or_else(|| vec![".".to_string()]);
        sources.push(Source {
            name,
            path: resolved_path,
            remote: raw.remote,
            folders,
            project: raw.project,
        });
    }

    Ok(MemexConfig {
        project_dir,
        config_path,
        sources,
    })
}

fn glob_md(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(glob_md(&path));
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
    out
}

fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    if p == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(p)
}
