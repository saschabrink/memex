//! `memex doctor` — print per-source diagnostics.
//!
//! Answers "why isn't my blueprint showing up?" by walking every source
//! the same way the normal enumeration does, but collecting reasons for
//! every skip: foreign `.git/` subtrees, noise directories
//! (`node_modules`, `target`, `_build`, `.next`), and `exclude` glob
//! hits. Also prints mount existence, remote reachability (with a 5s
//! ceiling), and index-file size.
//!
//! Exits 0 unless a source's mount doesn't exist or a remote fails the
//! reachability check — those exit 1, so `doctor` can be used in CI.

use anyhow::Result;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::config::{self, MemexConfig, SkipReason};

pub fn run(cfg: &MemexConfig) -> Result<()> {
    let mut had_error = false;

    println!("Project: {}", cfg.project_name);
    println!("Config:  {}", cfg.config_path.display());
    println!("Root:    {}", normalize_path(&cfg.project_root).display());
    println!();
    println!("Sources:");

    for source in &cfg.sources {
        println!();
        println!("  [{}]", source.name);
        let mount_rel = display_rel(&source.mount, &cfg.project_root);

        if !source.mount.exists() {
            println!("    mount     {mount_rel}  ✗ path does not exist");
            if source.remote.is_some() {
                println!("              run `memex sync` to clone from remote");
            }
            had_error = true;
        } else {
            match config::enumerate_source_with_diagnostics(source) {
                Ok(diag) => {
                    println!(
                        "    mount     {mount_rel}  ✓ {} indexed",
                        diag.matched.len()
                    );
                    if !diag.skipped.is_empty() {
                        println!("    skipped:");
                        for s in &diag.skipped {
                            let reason = match s.reason {
                                SkipReason::ForeignGit => "foreign .git/ subtree".to_string(),
                                SkipReason::NoiseDir(n) => format!("noise dir ({n})"),
                                SkipReason::ExcludeGlob => "matched exclude glob".to_string(),
                            };
                            println!("      {}/  —  {reason}", s.rel_path.display());
                        }
                    }
                }
                Err(e) => {
                    println!("    mount     {mount_rel}  ✗ enumeration failed: {e}");
                    had_error = true;
                }
            }
        }

        if source.prefix != source.name {
            if source.prefix.is_empty() {
                println!("    prefix    (empty — bare slugs)");
            } else {
                println!("    prefix    {}", source.prefix);
            }
        }

        if !source.include.is_empty() {
            println!("    include   {}", source.include.join(", "));
        }
        if !source.exclude.is_empty() {
            println!("    exclude   {}", source.exclude.join(", "));
        }
        if let Some(idx) = &source.index_filename {
            println!("    index     {idx}");
        }
        if source.readonly {
            println!("    readonly  true");
        }
        if let Some(remote) = &source.remote {
            let (ok, label) = check_remote(remote);
            if !ok {
                had_error = true;
            }
            println!("    remote    {remote}  {label}");
        }
    }

    if !cfg.also_scan.is_empty() {
        println!();
        println!("External files (also_scan — scanned for [[slug]] refs, not indexed):");
        match cfg.also_scan_files() {
            Ok(files) => {
                if files.is_empty() {
                    println!("  (no files match any pattern)");
                } else {
                    for glob in &cfg.also_scan {
                        println!("    pattern   {glob}");
                    }
                    println!("    matched:  {} file(s)", files.len());
                    for f in files.iter().take(10) {
                        println!("      {}", display_rel(f, &cfg.project_root));
                    }
                    if files.len() > 10 {
                        println!("      … and {} more", files.len() - 10);
                    }
                }
            }
            Err(e) => {
                println!("  ✗ scan failed: {e}");
                had_error = true;
            }
        }
    }

    println!();
    let db = cfg.db_path();
    let size = std::fs::metadata(&db).map(|m| m.len()).unwrap_or(0);
    if db.exists() {
        println!("Index: {} ({})", db.display(), format_size(size));
    } else {
        println!(
            "Index: {} (not yet built — will be created on first search/list)",
            db.display()
        );
    }

    if had_error {
        std::process::exit(1);
    }
    Ok(())
}

/// Best-effort: format a path relative to the project root, falling back
/// to the absolute display if it isn't a child.
fn display_rel(path: &Path, root: &Path) -> String {
    match path.strip_prefix(root) {
        Ok(p) if p.as_os_str().is_empty() => ".".to_string(),
        Ok(p) => p.display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

/// `git ls-remote` with a 5-second ceiling and no interactive prompts.
/// Returns (reachable, pretty_label).
fn check_remote(remote: &str) -> (bool, String) {
    let mut child = match Command::new("git")
        .args(["ls-remote", "--exit-code", "--quiet", remote, "HEAD"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return (false, format!("✗ cannot spawn git: {e}")),
    };

    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return (true, "✓ reachable".into()),
            Ok(Some(_)) => return (false, "✗ unreachable (non-zero exit)".into()),
            Ok(None) => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (false, "✗ timeout (>5s)".into());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return (false, format!("✗ {e}")),
        }
    }
}

/// Strip a trailing `.` component ("/some/path/." → "/some/path") so paths
/// derived from `root = "."` render cleanly.
fn normalize_path(p: &Path) -> std::path::PathBuf {
    if p.file_name().is_some_and(|n| n == ".") {
        p.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| p.to_path_buf())
    } else {
        p.to_path_buf()
    }
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
