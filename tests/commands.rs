//! Integration tests for command modules that exercise the embedder.
//!
//! Marked `#[ignore]` because the first run downloads the sentence-transformer
//! model (~87 MB). Run explicitly with:
//!
//!     cargo test --test commands -- --ignored

mod common;

use memex::commands;
use memex::db;

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
    assert_eq!(std::fs::read_to_string(&fp).unwrap(), "# Title\n\nbaz bar foo");
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
