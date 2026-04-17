mod common;

use memex::db::{self, EMBEDDING_DIM};

use common::mk_tmp;

fn fake_emb(seed: f32) -> Vec<f32> {
    (0..EMBEDDING_DIM).map(|i| seed + i as f32 * 0.0001).collect()
}

#[test]
fn setup_creates_schema() {
    let tmp = mk_tmp("memex-db-");
    let db_path = tmp.join("index.sqlite");
    let conn = db::connect(&db_path).unwrap();
    db::setup(&conn).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('blueprints','blueprint_embeddings')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 2);

    let version: i64 = conn
        .query_row("SELECT user_version FROM pragma_user_version", [], |r| r.get(0))
        .unwrap();
    assert_eq!(version, 2);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn upsert_and_get_roundtrip() {
    let tmp = mk_tmp("memex-db-");
    let mut conn = db::connect(&tmp.join("index.sqlite")).unwrap();
    db::setup(&conn).unwrap();

    db::upsert(
        &mut conn,
        "s/a",
        "Alpha",
        "/path/to/a.md",
        "s/folder",
        "# Alpha\n",
        "hash-a",
        &fake_emb(0.1),
    )
    .unwrap();

    let row = db::get(&conn, "s/a").unwrap().expect("row exists");
    assert_eq!(row.id, "s/a");
    assert_eq!(row.title, "Alpha");
    assert_eq!(row.folder, "s/folder");
    assert_eq!(row.content, "# Alpha\n");

    assert!(db::get(&conn, "s/missing").unwrap().is_none());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn upsert_overwrites_existing_row() {
    let tmp = mk_tmp("memex-db-");
    let mut conn = db::connect(&tmp.join("index.sqlite")).unwrap();
    db::setup(&conn).unwrap();

    db::upsert(&mut conn, "s/a", "Old", "/p", "s", "old", "h1", &fake_emb(0.1)).unwrap();
    db::upsert(&mut conn, "s/a", "New", "/p", "s", "new", "h2", &fake_emb(0.2)).unwrap();

    let row = db::get(&conn, "s/a").unwrap().unwrap();
    assert_eq!(row.title, "New");
    assert_eq!(row.content, "new");

    let state = db::index_state(&conn).unwrap();
    assert_eq!(state, vec![("s/a".to_string(), "h2".to_string())]);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn del_removes_row_and_embedding() {
    let tmp = mk_tmp("memex-db-");
    let mut conn = db::connect(&tmp.join("index.sqlite")).unwrap();
    db::setup(&conn).unwrap();
    db::upsert(&mut conn, "s/a", "A", "/p", "s", "c", "h", &fake_emb(0.1)).unwrap();

    db::del(&conn, "s/a").unwrap();
    assert!(db::get(&conn, "s/a").unwrap().is_none());
    let emb_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM blueprint_embeddings", [], |r| r.get(0))
        .unwrap();
    assert_eq!(emb_count, 0);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn list_all_filters_by_folder() {
    let tmp = mk_tmp("memex-db-");
    let mut conn = db::connect(&tmp.join("index.sqlite")).unwrap();
    db::setup(&conn).unwrap();

    db::upsert(&mut conn, "s/a", "A", "/p", "s/x", "c", "h", &fake_emb(0.1)).unwrap();
    db::upsert(&mut conn, "s/b", "B", "/p", "s/y", "c", "h", &fake_emb(0.1)).unwrap();
    db::upsert(&mut conn, "s/c", "C", "/p", "s/x", "c", "h", &fake_emb(0.1)).unwrap();

    let rows = db::list_all(&conn, &["s/x".to_string()]).unwrap();
    let ids: Vec<_> = rows.iter().map(|r| r.id.clone()).collect();
    assert_eq!(ids, vec!["s/a", "s/c"]);

    assert!(db::list_all(&conn, &[]).unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn search_ranks_by_cosine_distance() {
    let tmp = mk_tmp("memex-db-");
    let mut conn = db::connect(&tmp.join("index.sqlite")).unwrap();
    db::setup(&conn).unwrap();

    // Identical-to-query vector → distance 0. Others further away.
    let query = fake_emb(0.5);
    db::upsert(&mut conn, "s/match", "Match", "/p", "s", "c", "h", &query).unwrap();
    db::upsert(&mut conn, "s/far", "Far", "/p", "s", "c", "h", &fake_emb(-0.5)).unwrap();
    db::upsert(&mut conn, "s/other", "Other", "/p", "s", "c", "h", &fake_emb(0.1)).unwrap();

    let results = db::search(&conn, &query, &["s".to_string()], 3).unwrap();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].id, "s/match");
    assert!(results[0].distance < results[1].distance);
    assert!(results[1].distance <= results[2].distance);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn search_respects_limit() {
    let tmp = mk_tmp("memex-db-");
    let mut conn = db::connect(&tmp.join("index.sqlite")).unwrap();
    db::setup(&conn).unwrap();
    for i in 0..5 {
        db::upsert(
            &mut conn,
            &format!("s/{i}"),
            "T",
            "/p",
            "s",
            "c",
            "h",
            &fake_emb(i as f32 * 0.1),
        )
        .unwrap();
    }
    let results = db::search(&conn, &fake_emb(0.0), &["s".to_string()], 2).unwrap();
    assert_eq!(results.len(), 2);
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn cosine_similarity_basics() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    assert!((db::cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

    let c = vec![0.0, 1.0, 0.0];
    assert!(db::cosine_similarity(&a, &c).abs() < 1e-6);

    let zero = vec![0.0; 3];
    assert_eq!(db::cosine_similarity(&a, &zero), 0.0);
}

#[test]
fn setup_drops_legacy_schema() {
    let tmp = mk_tmp("memex-db-");
    let db_path = tmp.join("index.sqlite");
    let conn = db::connect(&db_path).unwrap();
    // simulate legacy v0/v1 schema without content_hash
    conn.execute_batch(
        "CREATE TABLE blueprints (id TEXT PRIMARY KEY, title TEXT);
         CREATE TABLE blueprint_embeddings (id TEXT PRIMARY KEY, embedding BLOB);",
    )
    .unwrap();
    conn.execute("INSERT INTO blueprints (id, title) VALUES ('old', 'Old')", [])
        .unwrap();

    db::setup(&conn).unwrap();

    // legacy row gone
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM blueprints", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);

    // new column present
    let has_col: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('blueprints') WHERE name='content_hash'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_col, 1);

    let _ = std::fs::remove_dir_all(&tmp);
}
