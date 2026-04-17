//! hooks.toml — pre-write / post-write hook dispatching.
//!
//! Discovery: one `hooks.toml` next to `memex.toml` (project-level), plus one
//! optional `hooks.toml` at the root of each source's `mount`. All merged in
//! the order: project → sources (in `memex.toml` order) → entries within each
//! file. First match per event wins.
//!
//! Pre-write hooks inject blueprint references ("read these before editing").
//! Post-write hooks emit text advice, optionally conditional on the existence
//! or absence of another file.

use anyhow::{anyhow, bail, Context, Result};
use fancy_regex::{Captures, Regex};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::config::MemexConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    PreWrite,
    PostWrite,
}

impl Event {
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "pre-write" => Ok(Event::PreWrite),
            "post-write" => Ok(Event::PostWrite),
            other => Err(anyhow!(
                "unknown event '{other}' (expected 'pre-write' or 'post-write')"
            )),
        }
    }

    pub fn claude_hook_name(self) -> &'static str {
        match self {
            Event::PreWrite => "PreToolUse",
            Event::PostWrite => "PostToolUse",
        }
    }
}

#[derive(Debug)]
pub struct Hook {
    pub pattern: Regex,
    pub pattern_src: String,
    /// Pre-write only. List of blueprint ids to inject.
    pub blueprints: Vec<String>,
    /// Post-write only. Text advice with `${0..n}` substitution.
    pub text: Option<String>,
    pub when_file_missing: Option<String>,
    pub when_file_exists: Option<String>,
    /// Source the hook came from (for diagnostics). `None` = project-level.
    pub source: Option<String>,
}

#[derive(Debug, Default)]
pub struct HookSet {
    pub pre_write: Vec<Hook>,
    pub post_write: Vec<Hook>,
}

#[derive(Debug)]
pub struct HookAdvice {
    /// For pre-write: blueprints to read.
    pub blueprints: Vec<String>,
    /// For post-write: advice text.
    pub text: Option<String>,
}

impl HookSet {
    /// Find the first matching hook for `file_path` under `event`.
    /// `file_path` should be relative to `project_root` (forward-slash).
    pub fn advise(
        &self,
        event: Event,
        file_path: &str,
        project_root: &Path,
    ) -> Option<HookAdvice> {
        let hooks = match event {
            Event::PreWrite => &self.pre_write,
            Event::PostWrite => &self.post_write,
        };
        for hook in hooks {
            let caps = match hook.pattern.captures(file_path) {
                Ok(Some(c)) => c,
                Ok(None) => continue,
                Err(_) => continue, // runtime regex error — treat as non-match
            };
            // Conditional checks (referencing the project root).
            if let Some(tmpl) = &hook.when_file_missing {
                let resolved = substitute(tmpl, &caps);
                if project_root.join(&resolved).exists() {
                    continue;
                }
            }
            if let Some(tmpl) = &hook.when_file_exists {
                let resolved = substitute(tmpl, &caps);
                if !project_root.join(&resolved).exists() {
                    continue;
                }
            }
            let blueprints = hook.blueprints.clone();
            let text = hook.text.as_ref().map(|t| substitute(t, &caps));
            return Some(HookAdvice { blueprints, text });
        }
        None
    }
}

/// Replace `${0}`, `${1}`, … with the corresponding regex capture group.
/// Missing groups are replaced with an empty string.
pub fn substitute(template: &str, caps: &Captures<'_>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(dollar) = rest.find('$') {
        out.push_str(&rest[..dollar]);
        let after = &rest[dollar..];
        if let Some(inner) = after.strip_prefix("${") {
            if let Some(close) = inner.find('}') {
                let idx_str = &inner[..close];
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if let Some(m) = caps.get(idx) {
                        out.push_str(m.as_str());
                    }
                    rest = &inner[close + 1..];
                    continue;
                }
            }
        }
        // Not a valid ${N} placeholder — keep the `$` literally, continue after it.
        out.push('$');
        rest = &after[1..];
    }
    out.push_str(rest);
    out
}

// ---------- loading ----------

#[derive(Deserialize, Debug)]
struct RawHooksFile {
    #[serde(rename = "pre-write", default)]
    pre_write: Vec<RawHook>,
    #[serde(rename = "post-write", default)]
    post_write: Vec<RawHook>,
}

#[derive(Deserialize, Debug)]
struct RawHook {
    pattern: String,
    blueprint: Option<StringOrList>,
    blueprints: Option<StringOrList>,
    text: Option<String>,
    when_file_missing: Option<String>,
    when_file_exists: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum StringOrList {
    Str(String),
    List(Vec<String>),
}

impl StringOrList {
    fn into_vec(self) -> Vec<String> {
        match self {
            StringOrList::Str(s) => vec![s],
            StringOrList::List(v) => v,
        }
    }
}

/// Load and merge all applicable `hooks.toml` files for a config.
/// Order: project-level first, then per-source in source order.
pub fn load(cfg: &MemexConfig) -> Result<HookSet> {
    let mut set = HookSet::default();

    // Project-level.
    let project_hooks = cfg.config_dir.join("hooks.toml");
    if project_hooks.exists() {
        load_into(&project_hooks, None, &mut set)
            .with_context(|| format!("loading {}", project_hooks.display()))?;
    }

    // Per-source.
    for source in &cfg.sources {
        let path = source.mount.join("hooks.toml");
        if path.exists() {
            load_into(&path, Some(source.name.clone()), &mut set)
                .with_context(|| format!("loading {}", path.display()))?;
        }
    }

    Ok(set)
}

fn load_into(path: &Path, source: Option<String>, out: &mut HookSet) -> Result<()> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let raw: RawHooksFile =
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;

    for raw_hook in raw.pre_write {
        let hook = build_hook(raw_hook, Event::PreWrite, source.clone())?;
        out.pre_write.push(hook);
    }
    for raw_hook in raw.post_write {
        let hook = build_hook(raw_hook, Event::PostWrite, source.clone())?;
        out.post_write.push(hook);
    }
    Ok(())
}

fn build_hook(raw: RawHook, event: Event, source: Option<String>) -> Result<Hook> {
    // Validate field combinations.
    match event {
        Event::PreWrite => {
            if raw.text.is_some() {
                bail!(
                    "[[pre-write]] hook with pattern '{}' has 'text' (allowed only on post-write)",
                    raw.pattern
                );
            }
            if raw.blueprint.is_none() && raw.blueprints.is_none() {
                bail!(
                    "[[pre-write]] hook with pattern '{}' needs 'blueprint' or 'blueprints'",
                    raw.pattern
                );
            }
            if raw.blueprint.is_some() && raw.blueprints.is_some() {
                bail!(
                    "[[pre-write]] hook with pattern '{}' has both 'blueprint' and 'blueprints' — pick one",
                    raw.pattern
                );
            }
        }
        Event::PostWrite => {
            if raw.blueprint.is_some() || raw.blueprints.is_some() {
                bail!(
                    "[[post-write]] hook with pattern '{}' has 'blueprint'/'blueprints' (allowed only on pre-write)",
                    raw.pattern
                );
            }
            if raw.text.is_none() {
                bail!(
                    "[[post-write]] hook with pattern '{}' needs 'text'",
                    raw.pattern
                );
            }
        }
    }

    let blueprints: Vec<String> = raw
        .blueprint
        .or(raw.blueprints)
        .map(|s| s.into_vec())
        .unwrap_or_default();

    let pattern = Regex::new(&raw.pattern)
        .with_context(|| format!("invalid regex in hook: {}", raw.pattern))?;

    Ok(Hook {
        pattern,
        pattern_src: raw.pattern,
        blueprints,
        text: raw.text,
        when_file_missing: raw.when_file_missing,
        when_file_exists: raw.when_file_exists,
        source,
    })
}

/// Normalize a file path into a project-relative forward-slash string.
/// If `file_path` is already relative, it's returned as-is (normalized slashes).
pub fn normalize_for_match(file_path: &str, project_root: &Path) -> String {
    let p = PathBuf::from(file_path);
    let rel = if p.is_absolute() {
        p.strip_prefix(project_root).unwrap_or(&p).to_path_buf()
    } else {
        p
    };
    rel.to_string_lossy().replace('\\', "/")
}
