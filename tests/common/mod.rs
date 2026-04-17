#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use memex::config::{self, MemexConfig};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TestEnv {
    pub tmp_dir: PathBuf,
    pub project_dir: PathBuf,
    /// Absolute mount path of the `testsource` source.
    pub source_mount: PathBuf,
    pub cfg: MemexConfig,
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.tmp_dir);
    }
}

/// Create a test project containing one source (`testsource`) mounted at
/// `<project>/blueprints`. The project root is a git repo (for commits).
pub fn create_env() -> TestEnv {
    let tmp_dir = mk_tmp("memex-test-");
    let project_dir = tmp_dir.join("project");
    std::fs::create_dir_all(&project_dir).unwrap();

    // Project is a git repo so commits work.
    git_init(&project_dir);

    let source_mount = project_dir.join("blueprints");
    std::fs::create_dir_all(&source_mount).unwrap();

    let config_path = project_dir.join("memex.toml");
    std::fs::write(
        &config_path,
        r#"project_name = "testproject"

[testsource]
mount = "blueprints"
"#,
    )
    .unwrap();

    let cfg = config::load(&project_dir, None).unwrap();
    cfg.ensure_initialized().unwrap();

    TestEnv {
        tmp_dir,
        project_dir,
        source_mount,
        cfg,
    }
}

pub fn mk_tmp(prefix: &str) -> PathBuf {
    let base = std::env::temp_dir();
    let unique = format!(
        "{prefix}{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let p = base.join(unique);
    std::fs::create_dir_all(&p).unwrap();
    p
}

pub fn git_init(dir: &Path) {
    run_git(dir, &["init", "-q"]);
    run_git(dir, &["config", "user.email", "test@memex.test"]);
    run_git(dir, &["config", "user.name", "Memex Test"]);
    std::fs::write(dir.join(".gitkeep"), "").unwrap();
    run_git(dir, &["add", ".gitkeep"]);
    run_git(dir, &["commit", "-q", "-m", "init"]);
}

pub fn run_git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git not available");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
