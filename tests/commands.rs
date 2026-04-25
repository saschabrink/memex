//! Integration tests for command modules that exercise the embedder.
//!
//! Marked `#[ignore]` because the first run downloads the sentence-transformer
//! model (~87 MB). Run explicitly with:
//!
//!     cargo test --test commands -- --ignored

mod common;

use common::{mk_tmp, run_git};
use memex::commands;
use memex::config;
use memex::db;
use memex::git;

use common::create_env;

#[test]
#[ignore]
fn write_creates_file_and_indexes() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/new-bp", "# New\n\nContent.").unwrap();

    let fp = env.source_mount.join("new-bp.md");
    assert!(fp.exists());
    assert_eq!(std::fs::read_to_string(&fp).unwrap(), "# New\n\nContent.");

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    let row = db::get(&conn, "testsource/new-bp").unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().title, "New");
}

#[test]
#[ignore]
fn list_returns_indexed_blueprints() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/alpha", "# Alpha").unwrap();
    commands::write::run(&env.cfg, "testsource/beta", "# Beta").unwrap();

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    let rows = db::list_all(&conn, &env.cfg.all_source_names()).unwrap();
    let ids: Vec<_> = rows.iter().map(|r| r.id.clone()).collect();
    assert!(ids.contains(&"testsource/alpha".to_string()));
    assert!(ids.contains(&"testsource/beta".to_string()));
}

#[test]
#[ignore]
fn edit_replaces_first_occurrence() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/bp", "# Title\n\nfoo bar foo").unwrap();
    commands::edit::run(&env.cfg, "testsource/bp", "foo", "baz").unwrap();

    let fp = env.source_mount.join("bp.md");
    assert_eq!(
        std::fs::read_to_string(&fp).unwrap(),
        "# Title\n\nbaz bar foo"
    );
}

#[test]
#[ignore]
fn delete_removes_file_and_index_row() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/gone", "# Gone").unwrap();
    commands::delete::run(&env.cfg, "testsource/gone").unwrap();

    let fp = env.source_mount.join("gone.md");
    assert!(!fp.exists());

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    assert!(db::get(&conn, "testsource/gone").unwrap().is_none());
}

#[test]
#[ignore]
fn move_within_same_source() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/old", "# Old\n\nStuff.").unwrap();
    commands::move_::run(&env.cfg, "testsource/old", "testsource/new").unwrap();

    let old = env.source_mount.join("old.md");
    let new = env.source_mount.join("new.md");
    assert!(!old.exists());
    assert!(new.exists());

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    assert!(db::get(&conn, "testsource/old").unwrap().is_none());
    assert!(db::get(&conn, "testsource/new").unwrap().is_some());
}

// ---------- sync: remote URL update ----------

#[test]
fn sync_updates_remote_url_when_config_changes() {
    let tmp = mk_tmp("memex-sync-remote-");

    // Set up two bare repos acting as remote-a and remote-b.
    let remote_a = tmp.join("remote-a.git");
    let remote_b = tmp.join("remote-b.git");
    for r in [&remote_a, &remote_b] {
        std::fs::create_dir_all(r).unwrap();
        run_git(r, &["init", "--bare", "-q"]);
    }
    // Seed remote-a with a commit so clone works.
    let seed = tmp.join("seed");
    std::fs::create_dir_all(&seed).unwrap();
    run_git(&seed, &["init", "-q"]);
    run_git(&seed, &["config", "user.email", "t@t.test"]);
    run_git(&seed, &["config", "user.name", "Test"]);
    std::fs::write(seed.join("note.md"), "# Note").unwrap();
    run_git(&seed, &["add", "."]);
    run_git(&seed, &["commit", "-q", "-m", "init"]);
    run_git(
        &seed,
        &["push", "-q", &remote_a.to_string_lossy(), "HEAD:main"],
    );

    let project = tmp.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let remote_a_url = remote_a.to_string_lossy().to_string();
    let remote_b_url = remote_b.to_string_lossy().to_string();

    // Initial memex.toml pointing at remote-a.
    std::fs::write(
        project.join("memex.toml"),
        format!("project_name = \"p\"\n\n[docs]\nremote = \"{remote_a_url}\"\nmount = \"docs\"\n"),
    )
    .unwrap();

    // First sync: clones from remote-a.
    let cfg = config::load(&project, None).unwrap();
    commands::sync::run(&cfg).unwrap();
    assert!(project.join("docs").exists());
    assert_eq!(
        git::get_remote_url(&project.join("docs"), "origin")
            .unwrap()
            .as_deref(),
        Some(remote_a_url.as_str())
    );

    // Change memex.toml to point at remote-b.
    std::fs::write(
        project.join("memex.toml"),
        format!("project_name = \"p\"\n\n[docs]\nremote = \"{remote_b_url}\"\nmount = \"docs\"\n"),
    )
    .unwrap();

    // Second sync: should update origin to remote-b.
    let cfg2 = config::load(&project, None).unwrap();
    commands::sync::run(&cfg2).unwrap();
    assert_eq!(
        git::get_remote_url(&project.join("docs"), "origin")
            .unwrap()
            .as_deref(),
        Some(remote_b_url.as_str())
    );
}

#[test]
fn sync_leaves_remote_unchanged_when_url_matches() {
    let tmp = mk_tmp("memex-sync-same-");

    let remote = tmp.join("remote.git");
    std::fs::create_dir_all(&remote).unwrap();
    run_git(&remote, &["init", "--bare", "-q"]);

    let seed = tmp.join("seed");
    std::fs::create_dir_all(&seed).unwrap();
    run_git(&seed, &["init", "-q"]);
    run_git(&seed, &["config", "user.email", "t@t.test"]);
    run_git(&seed, &["config", "user.name", "Test"]);
    std::fs::write(seed.join("note.md"), "# Note").unwrap();
    run_git(&seed, &["add", "."]);
    run_git(&seed, &["commit", "-q", "-m", "init"]);
    run_git(
        &seed,
        &["push", "-q", &remote.to_string_lossy(), "HEAD:main"],
    );

    let project = tmp.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let remote_url = remote.to_string_lossy().to_string();

    std::fs::write(
        project.join("memex.toml"),
        format!("project_name = \"p\"\n\n[docs]\nremote = \"{remote_url}\"\nmount = \"docs\"\n"),
    )
    .unwrap();

    let cfg = config::load(&project, None).unwrap();
    commands::sync::run(&cfg).unwrap();

    // Sync again with same URL — remote should stay the same.
    let cfg2 = config::load(&project, None).unwrap();
    commands::sync::run(&cfg2).unwrap();

    assert_eq!(
        git::get_remote_url(&project.join("docs"), "origin")
            .unwrap()
            .as_deref(),
        Some(remote_url.as_str())
    );
}

#[test]
#[ignore]
fn broken_refs_reports_unresolved_slugs() {
    let env = create_env();
    commands::write::run(
        &env.cfg,
        "testsource/a",
        "# A\n\nLinks to [[testsource/missing]].",
    )
    .unwrap();
    // Just verify it doesn't error — stdout capture is awkward with println!.
    commands::broken_refs::run(&env.cfg).unwrap();
}
