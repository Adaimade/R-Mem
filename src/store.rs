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
}

/// SQLite-backed vector store with embedded vectors.
pub struct MemoryStore {
    db: Arc<Mutex<Connection>>,
}

impl MemoryStore {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open memory DB")?;

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

             CREATE TABLE IF NOT EXISTS history (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 memory_id TEXT NOT NULL,
                 action TEXT NOT NULL,
                 old_text TEXT,
                 new_text TEXT,
                 created_at TEXT DEFAULT (datetime('now'))
             );",
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

        Ok(())
    }

    /// Update an existing memory.
    pub async fn update(
        &self,
        id: &str,
        text: &str,
        embedding_vec: &[f32],
    ) -> Result<()> {
        let db = self.db.lock().await;
        let blob = embedding_to_blob(embedding_vec);

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

        Ok(())
    }

    /// Delete a memory by ID.
    pub async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db.lock().await;

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
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);

        Ok(results)
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
        Ok(count as u64)
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
