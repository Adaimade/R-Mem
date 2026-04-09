use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::embedding;

/// A single memory record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub user_id: String,
    pub text: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Search result with score.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResult {
    pub id: String,
    pub text: String,
    pub score: f32,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>, // "active", "archive", or "graph"
}

/// An archived memory record.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ArchivedRecord {
    pub id: String,
    pub user_id: String,
    pub text: String,
    pub reason: String,        // "DELETED" or "SUPERSEDED"
    pub superseded_by: Option<String>,
    pub archived_at: String,
    pub original_created_at: String,
}

/// SQLite-backed vector store with embedded vectors.
pub struct MemoryStore {
    db: Arc<Mutex<Connection>>,
}

impl MemoryStore {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open memory DB")?;

        // WAL mode: allows concurrent reads while writing
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                 id TEXT PRIMARY KEY,
                 user_id TEXT NOT NULL,
                 text TEXT NOT NULL,
                 embedding BLOB,
                 created_at TEXT DEFAULT (datetime('now')),
                 updated_at TEXT DEFAULT (datetime('now'))
             );
             CREATE INDEX IF NOT EXISTS idx_memories_user ON memories(user_id);

             -- FTS5 full-text index for pre-filtering before vector search
             CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                 text, content='memories', content_rowid='rowid'
             );

             CREATE TABLE IF NOT EXISTS history (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 memory_id TEXT NOT NULL,
                 action TEXT NOT NULL,
                 old_text TEXT,
                 new_text TEXT,
                 created_at TEXT DEFAULT (datetime('now'))
             );

             CREATE TABLE IF NOT EXISTS archive (
                 id TEXT PRIMARY KEY,
                 user_id TEXT NOT NULL,
                 text TEXT NOT NULL,
                 embedding BLOB,
                 reason TEXT NOT NULL,
                 superseded_by TEXT,
                 archived_at TEXT DEFAULT (datetime('now')),
                 original_created_at TEXT
             );
             CREATE INDEX IF NOT EXISTS idx_archive_user ON archive(user_id);",
        )?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    /// Insert a new memory with embedding.
    pub async fn add(
        &self,
        id: &str,
        user_id: &str,
        text: &str,
        embedding: &[f32],
    ) -> Result<()> {
        let db = self.db.lock().await;
        let blob = embedding_to_blob(embedding);

        db.execute(
            "INSERT INTO memories (id, user_id, text, embedding) VALUES (?1, ?2, ?3, ?4)",
            params![id, user_id, text, blob],
        )
        .context("Failed to insert memory")?;

        db.execute(
            "INSERT INTO history (memory_id, action, new_text) VALUES (?1, 'ADD', ?2)",
            params![id, text],
        )?;

        // Update FTS index
        db.execute(
            "INSERT INTO memories_fts(rowid, text) SELECT rowid, text FROM memories WHERE id = ?1",
            [id],
        ).ok(); // non-fatal if FTS fails

        Ok(())
    }

    /// Update an existing memory. The old version is archived.
    pub async fn update(
        &self,
        id: &str,
        text: &str,
        embedding_vec: &[f32],
    ) -> Result<()> {
        let db = self.db.lock().await;
        let blob = embedding_to_blob(embedding_vec);

        // Archive the old version before overwriting
        db.execute(
            "INSERT OR IGNORE INTO archive (id, user_id, text, embedding, reason, superseded_by, original_created_at)
             SELECT id || ':' || CAST(RANDOM() AS TEXT), user_id, text, embedding, 'SUPERSEDED', ?1, created_at
             FROM memories WHERE id = ?2",
            params![id, id],
        )?;

        let old_text: Option<String> = db
            .query_row("SELECT text FROM memories WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .ok();

        db.execute(
            "UPDATE memories SET text = ?1, embedding = ?2, updated_at = datetime('now') WHERE id = ?3",
            params![text, blob, id],
        )
        .context("Failed to update memory")?;

        db.execute(
            "INSERT INTO history (memory_id, action, old_text, new_text) VALUES (?1, 'UPDATE', ?2, ?3)",
            params![id, old_text, text],
        )?;

        // Rebuild FTS for this row
        db.execute(
            "INSERT INTO memories_fts(memories_fts, rowid, text) VALUES('delete', (SELECT rowid FROM memories WHERE id = ?1), ?2)",
            params![id, old_text],
        ).ok();
        db.execute(
            "INSERT INTO memories_fts(rowid, text) SELECT rowid, text FROM memories WHERE id = ?1",
            [id],
        ).ok();

        Ok(())
    }

    /// Delete a memory by ID. The deleted memory is archived.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db.lock().await;

        // Archive before deleting
        db.execute(
            "INSERT OR IGNORE INTO archive (id, user_id, text, embedding, reason, original_created_at)
             SELECT id, user_id, text, embedding, 'DELETED', created_at
             FROM memories WHERE id = ?1",
            [id],
        )?;

        let old_text: Option<String> = db
            .query_row("SELECT text FROM memories WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .ok();

        db.execute("DELETE FROM memories WHERE id = ?1", [id])?;

        db.execute(
            "INSERT INTO history (memory_id, action, old_text) VALUES (?1, 'DELETE', ?2)",
            params![id, old_text],
        )?;

        Ok(())
    }

    /// Get all memories for a user.
    pub async fn get_all(&self, user_id: &str) -> Result<Vec<MemoryRecord>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, user_id, text, created_at, updated_at FROM memories WHERE user_id = ?1 ORDER BY updated_at DESC",
        )?;

        let rows = stmt
            .query_map([user_id], |row| {
                Ok(MemoryRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    text: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Get a single memory by ID.
    pub async fn get(&self, id: &str) -> Result<Option<MemoryRecord>> {
        let db = self.db.lock().await;
        let result = db
            .query_row(
                "SELECT id, user_id, text, created_at, updated_at FROM memories WHERE id = ?1",
                [id],
                |row| {
                    Ok(MemoryRecord {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        text: row.get(2)?,
                        created_at: row.get(3)?,
                        updated_at: row.get(4)?,
                    })
                },
            )
            .ok();
        Ok(result)
    }

    /// Vector similarity search. Loads all user embeddings and computes cosine similarity.
    pub async fn search(
        &self,
        user_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, user_id, text, embedding FROM memories WHERE user_id = ?1",
        )?;

        let mut results: Vec<SearchResult> = stmt
            .query_map([user_id], |row| {
                let id: String = row.get(0)?;
                let uid: String = row.get(1)?;
                let text: String = row.get(2)?;
                let blob: Vec<u8> = row.get(3)?;
                Ok((id, uid, text, blob))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, uid, text, blob)| {
                let emb = blob_to_embedding(&blob);
                let score = embedding::cosine_similarity(query_embedding, &emb);
                SearchResult {
                    id,
                    text,
                    score,
                    user_id: uid,
                    source: None,
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
    }

    /// Full-text search using FTS5. Returns matching memory IDs for pre-filtering.
    pub async fn fts_search(&self, user_id: &str, query: &str, limit: usize) -> Result<Vec<String>> {
        let db = self.db.lock().await;

        // Tokenize query into FTS5 terms (OR logic)
        let terms: Vec<&str> = query.split_whitespace()
            .filter(|w| w.len() > 2)
            .collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = terms.join(" OR ");

        let sql = "SELECT m.id FROM memories m
                    JOIN memories_fts f ON m.rowid = f.rowid
                    WHERE m.user_id = ?1 AND memories_fts MATCH ?2
                    LIMIT ?3";

        let mut stmt = db.prepare(sql)?;
        let ids: Vec<String> = stmt
            .query_map(params![user_id, fts_query, limit], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(ids)
    }

    /// Get existing memories as (id, text) pairs for dedup.
    pub async fn get_existing_for_dedup(&self, user_id: &str) -> Result<Vec<(String, String)>> {
        let db = self.db.lock().await;
        let mut stmt =
            db.prepare("SELECT id, text FROM memories WHERE user_id = ?1")?;

        let rows = stmt
            .query_map([user_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Get change history for a memory.
    pub async fn history(&self, id: &str) -> Result<Vec<serde_json::Value>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT action, old_text, new_text, created_at FROM history WHERE memory_id = ?1 ORDER BY id ASC",
        )?;

        let rows = stmt
            .query_map([id], |row| {
                Ok(serde_json::json!({
                    "action": row.get::<_, String>(0)?,
                    "old_text": row.get::<_, Option<String>>(1)?,
                    "new_text": row.get::<_, Option<String>>(2)?,
                    "timestamp": row.get::<_, String>(3)?,
                }))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Delete all memories for a user.
    pub async fn reset(&self, user_id: &str) -> Result<u64> {
        let db = self.db.lock().await;
        let count = db.execute("DELETE FROM memories WHERE user_id = ?1", [user_id])?;
        db.execute("DELETE FROM archive WHERE user_id = ?1", [user_id])?;
        Ok(count as u64)
    }

    // ── Archive ─────────────────────────────────────────────────────

    /// Search archive by vector similarity (fallback when active search is insufficient).
    pub async fn search_archive(
        &self,
        user_id: &str,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, user_id, text, embedding FROM archive WHERE user_id = ?1 AND embedding IS NOT NULL",
        )?;

        let mut results: Vec<SearchResult> = stmt
            .query_map([user_id], |row| {
                let id: String = row.get(0)?;
                let uid: String = row.get(1)?;
                let text: String = row.get(2)?;
                let blob: Vec<u8> = row.get(3)?;
                Ok((id, uid, text, blob))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, uid, text, blob)| {
                let emb = blob_to_embedding(&blob);
                let score = embedding::cosine_similarity(query_embedding, &emb);
                SearchResult {
                    id,
                    text,
                    score,
                    user_id: uid,
                    source: Some("archive".to_string()),
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }

    /// Count archive entries for a user.
    pub async fn archive_count(&self, user_id: &str) -> Result<usize> {
        let db = self.db.lock().await;
        let count: i64 = db.query_row(
            "SELECT COUNT(*) FROM archive WHERE user_id = ?1",
            [user_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get all archived memories for a user.
    pub async fn get_archive(&self, user_id: &str) -> Result<Vec<ArchivedRecord>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT id, user_id, text, reason, superseded_by, archived_at, original_created_at
             FROM archive WHERE user_id = ?1 ORDER BY archived_at DESC",
        )?;

        let rows = stmt
            .query_map([user_id], |row| {
                Ok(ArchivedRecord {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    text: row.get(2)?,
                    reason: row.get(3)?,
                    superseded_by: row.get(4)?,
                    archived_at: row.get(5)?,
                    original_created_at: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Delete old archive entries, keeping only the most recent `keep` entries per user.
    pub async fn compact_archive(&self, user_id: &str, keep: usize) -> Result<usize> {
        let db = self.db.lock().await;
        let deleted = db.execute(
            "DELETE FROM archive WHERE user_id = ?1 AND id NOT IN (
                SELECT id FROM archive WHERE user_id = ?1 ORDER BY archived_at DESC LIMIT ?2
            )",
            params![user_id, keep],
        )?;
        Ok(deleted)
    }
}

fn embedding_to_blob(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

fn blob_to_embedding(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_embedding(seed: f32) -> Vec<f32> {
        vec![seed, seed * 0.5, seed * 0.3]
    }

    #[test]
    fn embedding_blob_roundtrip() {
        let emb = vec![1.0_f32, -2.5, 3.14, 0.0];
        let blob = embedding_to_blob(&emb);
        let back = blob_to_embedding(&blob);
        assert_eq!(emb, back);
    }

    #[tokio::test]
    async fn store_add_and_get() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "likes sushi", &emb).await.unwrap();

        let record = store.get("id1").await.unwrap().unwrap();
        assert_eq!(record.text, "likes sushi");
        assert_eq!(record.user_id, "alice");
    }

    #[tokio::test]
    async fn store_update_records_history() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "likes sushi", &emb).await.unwrap();
        store.update("id1", "loves sushi", &emb).await.unwrap();

        let record = store.get("id1").await.unwrap().unwrap();
        assert_eq!(record.text, "loves sushi");

        let hist = store.history("id1").await.unwrap();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0]["action"], "ADD");
        assert_eq!(hist[1]["action"], "UPDATE");
    }

    #[tokio::test]
    async fn store_delete_removes_record() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "likes sushi", &emb).await.unwrap();
        store.delete("id1").await.unwrap();

        assert!(store.get("id1").await.unwrap().is_none());

        let hist = store.history("id1").await.unwrap();
        assert_eq!(hist.last().unwrap()["action"], "DELETE");
    }

    #[tokio::test]
    async fn store_search_returns_top_k() {
        let store = MemoryStore::open(":memory:").unwrap();
        // Add 3 memories with different embeddings
        store.add("id1", "alice", "likes sushi", &[1.0, 0.0, 0.0]).await.unwrap();
        store.add("id2", "alice", "likes pizza", &[0.9, 0.1, 0.0]).await.unwrap();
        store.add("id3", "alice", "works at google", &[0.0, 0.0, 1.0]).await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search("alice", &query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        // First result should be the most similar (id1)
        assert_eq!(results[0].id, "id1");
    }

    #[tokio::test]
    async fn store_reset_clears_all() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "fact 1", &emb).await.unwrap();
        store.add("id2", "alice", "fact 2", &emb).await.unwrap();

        let count = store.reset("alice").await.unwrap();
        assert_eq!(count, 2);
        assert!(store.get_all("alice").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn delete_archives_memory() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "likes sushi", &emb).await.unwrap();
        store.delete("id1").await.unwrap();

        // Active memory gone
        assert!(store.get("id1").await.unwrap().is_none());

        // But archived
        let archive = store.get_archive("alice").await.unwrap();
        assert_eq!(archive.len(), 1);
        assert_eq!(archive[0].text, "likes sushi");
        assert_eq!(archive[0].reason, "DELETED");
    }

    #[tokio::test]
    async fn update_archives_old_version() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "likes sushi", &emb).await.unwrap();
        store.update("id1", "loves sushi", &emb).await.unwrap();

        // Active has new version
        let record = store.get("id1").await.unwrap().unwrap();
        assert_eq!(record.text, "loves sushi");

        // Archive has old version
        let archive = store.get_archive("alice").await.unwrap();
        assert_eq!(archive.len(), 1);
        assert_eq!(archive[0].text, "likes sushi");
        assert_eq!(archive[0].reason, "SUPERSEDED");
    }

    #[tokio::test]
    async fn archive_search_finds_deleted() {
        let store = MemoryStore::open(":memory:").unwrap();
        store.add("id1", "alice", "likes sushi", &[1.0, 0.0, 0.0]).await.unwrap();
        store.delete("id1").await.unwrap();

        let query = vec![1.0, 0.0, 0.0];
        let results = store.search_archive("alice", &query, 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "likes sushi");
        assert_eq!(results[0].source.as_deref(), Some("archive"));
    }

    #[tokio::test]
    async fn reset_clears_archive() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "fact 1", &emb).await.unwrap();
        store.delete("id1").await.unwrap();
        assert_eq!(store.archive_count("alice").await.unwrap(), 1);

        store.reset("alice").await.unwrap();
        assert_eq!(store.archive_count("alice").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn store_user_isolation() {
        let store = MemoryStore::open(":memory:").unwrap();
        let emb = fake_embedding(1.0);
        store.add("id1", "alice", "alice fact", &emb).await.unwrap();
        store.add("id2", "bob", "bob fact", &emb).await.unwrap();

        let alice = store.get_all("alice").await.unwrap();
        assert_eq!(alice.len(), 1);
        assert_eq!(alice[0].text, "alice fact");
    }
}
