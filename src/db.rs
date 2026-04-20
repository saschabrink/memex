use anyhow::{Context, Result};
use rusqlite::{params_from_iter, Connection};
use std::path::Path;

pub const EMBEDDING_DIM: usize = 384;

#[derive(Debug)]
pub struct BlueprintRow {
    pub id: String,
    pub title: String,
    pub path: String,
    pub folder: String,
    pub content: String,
}

pub fn connect(db_path: &Path) -> Result<Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let conn =
        Connection::open(db_path).with_context(|| format!("opening {}", db_path.display()))?;
    Ok(conn)
}

const SCHEMA_VERSION: i64 = 2;

pub fn setup(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("SELECT user_version FROM pragma_user_version", [], |r| {
        r.get(0)
    })?;
    if version < SCHEMA_VERSION {
        conn.execute_batch(
            "DROP TABLE IF EXISTS blueprints;
             DROP TABLE IF EXISTS blueprint_embeddings;",
        )?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS blueprints (
            id            TEXT PRIMARY KEY,
            title         TEXT NOT NULL,
            path          TEXT NOT NULL,
            folder        TEXT NOT NULL,
            content       TEXT NOT NULL,
            content_hash  TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS blueprint_embeddings (
            id        TEXT PRIMARY KEY,
            embedding BLOB NOT NULL
        );",
    )?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

pub fn index_state(conn: &Connection) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT id, content_hash FROM blueprints")?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn list_all(conn: &Connection, folders: &[String]) -> Result<Vec<BlueprintRow>> {
    if folders.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; folders.len()].join(", ");
    let sql = format!(
        "SELECT id, title, path, folder, content FROM blueprints
         WHERE folder IN ({placeholders}) ORDER BY id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(folders.iter()), |row| {
            Ok(BlueprintRow {
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get(2)?,
                folder: row.get(3)?,
                content: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[derive(Debug)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub path: String,
    pub folder: String,
    pub distance: f32,
}

pub fn search(
    conn: &Connection,
    query: &[f32],
    folders: &[String],
    limit: usize,
) -> Result<Vec<SearchResult>> {
    if folders.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; folders.len()].join(", ");
    let sql = format!(
        "SELECT e.id, e.embedding, b.title, b.path, b.folder
         FROM blueprint_embeddings e
         JOIN blueprints b ON e.id = b.id
         WHERE b.folder IN ({placeholders})"
    );
    let mut stmt = conn.prepare(&sql)?;
    let raw: Vec<(String, Vec<u8>, String, String, String)> = stmt
        .query_map(params_from_iter(folders.iter()), |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut results: Vec<SearchResult> = raw
        .into_iter()
        .map(|(id, blob, title, path, folder)| {
            let emb = deserialize(&blob);
            let distance = 1.0 - cosine_similarity(query, &emb);
            SearchResult {
                id,
                title,
                path,
                folder,
                distance,
            }
        })
        .collect();
    results.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);
    Ok(results)
}

fn deserialize(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let (mut dot, mut na, mut nb) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

pub fn upsert(
    conn: &mut Connection,
    id: &str,
    title: &str,
    file_path: &str,
    folder: &str,
    content: &str,
    content_hash: &str,
    embedding: &[f32],
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR REPLACE INTO blueprints (id, title, path, folder, content, content_hash)
         VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![id, title, file_path, folder, content, content_hash],
    )?;
    tx.execute("DELETE FROM blueprint_embeddings WHERE id = ?", [id])?;
    tx.execute(
        "INSERT INTO blueprint_embeddings (id, embedding) VALUES (?, ?)",
        rusqlite::params![id, serialize(embedding)],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn del(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM blueprint_embeddings WHERE id = ?", [id])?;
    conn.execute("DELETE FROM blueprints WHERE id = ?", [id])?;
    Ok(())
}

fn serialize(embedding: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(embedding.len() * 4);
    for f in embedding {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

pub fn get(conn: &Connection, id: &str) -> Result<Option<BlueprintRow>> {
    let mut stmt =
        conn.prepare("SELECT id, title, path, folder, content FROM blueprints WHERE id = ?")?;
    let row = stmt
        .query_row([id], |row| {
            Ok(BlueprintRow {
                id: row.get(0)?,
                title: row.get(1)?,
                path: row.get(2)?,
                folder: row.get(3)?,
                content: row.get(4)?,
            })
        })
        .ok();
    Ok(row)
}
