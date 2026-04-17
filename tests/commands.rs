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
    commands::write::run(&env.cfg, "testsource/test/tech/new-bp", "# New\n\nContent.").unwrap();

    let fp = env.source_dir.join("test").join("tech").join("new-bp.md");
    assert!(fp.exists());
    assert_eq!(std::fs::read_to_string(&fp).unwrap(), "# New\n\nContent.");

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    let row = db::get(&conn, "testsource/test/tech/new-bp").unwrap();
    assert!(row.is_some());
    assert_eq!(row.unwrap().title, "New");
}

#[test]
#[ignore]
fn list_returns_indexed_blueprints() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/test/tech/alpha", "# Alpha").unwrap();
    commands::write::run(&env.cfg, "testsource/test/tech/beta", "# Beta").unwrap();

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    let rows = db::list_all(&conn, &env.cfg.all_folders()).unwrap();
    let ids: Vec<_> = rows.iter().map(|r| r.id.clone()).collect();
    assert!(ids.contains(&"testsource/test/tech/alpha".to_string()));
    assert!(ids.contains(&"testsource/test/tech/beta".to_string()));
}

#[test]
#[ignore]
fn edit_replaces_first_occurrence() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/test/tech/bp", "# Title\n\nfoo bar foo").unwrap();
    commands::edit::run(&env.cfg, "testsource/test/tech/bp", "foo", "baz").unwrap();

    let fp = env.source_dir.join("test").join("tech").join("bp.md");
    assert_eq!(std::fs::read_to_string(&fp).unwrap(), "# Title\n\nbaz bar foo");
}

#[test]
#[ignore]
fn delete_removes_file_and_index_row() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/test/tech/gone", "# Gone").unwrap();
    commands::delete::run(&env.cfg, "testsource/test/tech/gone").unwrap();

    let fp = env.source_dir.join("test").join("tech").join("gone.md");
    assert!(!fp.exists());

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    assert!(db::get(&conn, "testsource/test/tech/gone").unwrap().is_none());
}

#[test]
#[ignore]
fn move_within_same_source() {
    let env = create_env();
    commands::write::run(&env.cfg, "testsource/test/tech/old", "# Old\n\nStuff.").unwrap();
    commands::move_::run(
        &env.cfg,
        "testsource/test/tech/old",
        "testsource/test/tech/new",
    )
    .unwrap();

    let old = env.source_dir.join("test").join("tech").join("old.md");
    let new = env.source_dir.join("test").join("tech").join("new.md");
    assert!(!old.exists());
    assert!(new.exists());

    let conn = db::connect(&env.cfg.db_path()).unwrap();
    db::setup(&conn).unwrap();
    assert!(db::get(&conn, "testsource/test/tech/old").unwrap().is_none());
    assert!(db::get(&conn, "testsource/test/tech/new").unwrap().is_some());
}

#[test]
#[ignore]
fn broken_refs_reports_unresolved_slugs() {
    let env = create_env();
    commands::write::run(
        &env.cfg,
        "testsource/test/tech/a",
        "# A\n\nLinks to [[missing-slug]].",
    )
    .unwrap();
    // Just verify it doesn't error — stdout capture is awkward with println!.
    commands::broken_refs::run(&env.cfg).unwrap();
}
