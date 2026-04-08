use anyhow::{Context, Result};
use rusqlite::params;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// A relationship in the graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Relation {
    pub source: String,
    pub relation: String,
    pub destination: String,
    pub mentions: i64,
    pub valid: bool,
}

/// SQLite-backed graph store (lightweight alternative to Neo4j).
/// Uses two tables: entities and relations, with soft-delete support.
pub struct GraphStore {
    db: Arc<Mutex<rusqlite::Connection>>,
}

impl GraphStore {
    pub fn open(path: &str) -> Result<Self> {
        // Use the same DB file, different tables
        let conn = rusqlite::Connection::open(path).context("Failed to open graph DB")?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entities (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 user_id TEXT NOT NULL,
                 name TEXT NOT NULL,
                 entity_type TEXT DEFAULT '',
                 mentions INTEGER DEFAULT 1,
                 created_at TEXT DEFAULT (datetime('now')),
                 UNIQUE(user_id, name)
             );

             CREATE TABLE IF NOT EXISTS relations (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 user_id TEXT NOT NULL,
                 source TEXT NOT NULL,
                 relation TEXT NOT NULL,
                 destination TEXT NOT NULL,
                 mentions INTEGER DEFAULT 1,
                 valid INTEGER DEFAULT 1,
                 created_at TEXT DEFAULT (datetime('now')),
                 updated_at TEXT DEFAULT (datetime('now')),
                 invalidated_at TEXT,
                 UNIQUE(user_id, source, relation, destination)
             );

             CREATE INDEX IF NOT EXISTS idx_entities_user ON entities(user_id);
             CREATE INDEX IF NOT EXISTS idx_relations_user ON relations(user_id);
             CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(user_id, source);
             CREATE INDEX IF NOT EXISTS idx_relations_dest ON relations(user_id, destination);",
        )?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
        })
    }

    /// Add or update a relation. If the exact triple exists, increment mentions.
    /// If a conflicting relation exists (same source+relation, different destination),
    /// soft-delete the old one.
    pub async fn add_relation(
        &self,
        user_id: &str,
        source: &str,
        relation: &str,
        destination: &str,
    ) -> Result<()> {
        let db = self.db.lock().await;

        // Ensure entities exist
        db.execute(
            "INSERT INTO entities (user_id, name) VALUES (?1, ?2)
             ON CONFLICT(user_id, name) DO UPDATE SET mentions = mentions + 1",
            params![user_id, source],
        )?;
        db.execute(
            "INSERT INTO entities (user_id, name) VALUES (?1, ?2)
             ON CONFLICT(user_id, name) DO UPDATE SET mentions = mentions + 1",
            params![user_id, destination],
        )?;

        // Soft-delete conflicting relations
        // e.g., if "小明 lives_in Tokyo" exists and we add "小明 lives_in London"
        // → soft-delete the Tokyo one (only for single-value relations)
        // For multi-value relations like "likes", don't delete
        let is_single_value = !is_multi_value_relation(relation);
        if is_single_value {
            db.execute(
                "UPDATE relations SET valid = 0, invalidated_at = datetime('now')
                 WHERE user_id = ?1 AND source = ?2 AND relation = ?3
                 AND destination != ?4 AND valid = 1",
                params![user_id, source, relation, destination],
            )?;
        }

        // Upsert the relation
        db.execute(
            "INSERT INTO relations (user_id, source, relation, destination)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(user_id, source, relation, destination)
             DO UPDATE SET mentions = mentions + 1, valid = 1,
                          updated_at = datetime('now'), invalidated_at = NULL",
            params![user_id, source, relation, destination],
        )?;

        Ok(())
    }

    /// Search for relations involving entities mentioned in the query.
    pub async fn search(&self, user_id: &str, query: &str) -> Result<Vec<Relation>> {
        self.search_with_limit(user_id, query, 20).await
    }

    pub async fn search_with_limit(&self, user_id: &str, query: &str, limit: usize) -> Result<Vec<Relation>> {
        let db = self.db.lock().await;

        // Tokenize query into words for matching
        let words: Vec<String> = query
            .split_whitespace()
            .map(|w| w.to_lowercase().replace(['，', '。', '？', '！', ',', '.', '?', '!'], ""))
            .filter(|w| w.len() > 1)
            .collect();

        if words.is_empty() {
            return Ok(Vec::new());
        }

        // Build a LIKE query for each word against source and destination
        let mut conditions = Vec::new();
        let mut query_params: Vec<String> = vec![user_id.to_string()];

        for word in &words {
            let idx = query_params.len();
            query_params.push(format!("%{word}%"));
            conditions.push(format!(
                "(LOWER(source) LIKE ?{} OR LOWER(destination) LIKE ?{})",
                idx + 1,
                idx + 1
            ));
        }

        let where_clause = conditions.join(" OR ");
        let sql = format!(
            "SELECT source, relation, destination, mentions, valid
             FROM relations
             WHERE user_id = ?1 AND valid = 1 AND ({where_clause})
             ORDER BY mentions DESC
             LIMIT {limit}"
        );

        let mut stmt = db.prepare(&sql)?;

        let params_refs: Vec<&dyn rusqlite::types::ToSql> = query_params
            .iter()
            .map(|s| s as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok(Relation {
                    source: row.get(0)?,
                    relation: row.get(1)?,
                    destination: row.get(2)?,
                    mentions: row.get(3)?,
                    valid: row.get::<_, i32>(4)? == 1,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Get all relations for a user.
    pub async fn get_all(&self, user_id: &str) -> Result<Vec<Relation>> {
        let db = self.db.lock().await;
        let mut stmt = db.prepare(
            "SELECT source, relation, destination, mentions, valid
             FROM relations WHERE user_id = ?1 AND valid = 1
             ORDER BY updated_at DESC",
        )?;

        let rows = stmt
            .query_map([user_id], |row| {
                Ok(Relation {
                    source: row.get(0)?,
                    relation: row.get(1)?,
                    destination: row.get(2)?,
                    mentions: row.get(3)?,
                    valid: row.get::<_, i32>(4)? == 1,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(rows)
    }

    /// Delete all graph data for a user.
    pub async fn reset(&self, user_id: &str) -> Result<()> {
        let db = self.db.lock().await;
        db.execute("DELETE FROM relations WHERE user_id = ?1", [user_id])?;
        db.execute("DELETE FROM entities WHERE user_id = ?1", [user_id])?;
        Ok(())
    }
}

/// Heuristic: multi-value relations allow multiple destinations.
/// e.g., "likes pizza" AND "likes burger" should both exist.
/// Single-value: "lives_in", "works_at", "born_in" — only one at a time.
fn is_multi_value_relation(relation: &str) -> bool {
    let lower = relation.to_lowercase();
    let multi = [
        "likes", "loves", "enjoys", "prefers", "uses", "knows",
        "speaks", "has", "owns", "plays", "watches", "reads",
        "friends_with", "colleague_of",
    ];
    multi.iter().any(|m| lower.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_graph() -> GraphStore {
        GraphStore::open(":memory:").unwrap()
    }

    #[tokio::test]
    async fn add_and_get_relation() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "works_at", "Google").await.unwrap();

        let rels = g.get_all("alice").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].source, "Alice");
        assert_eq!(rels[0].relation, "works_at");
        assert_eq!(rels[0].destination, "Google");
    }

    #[tokio::test]
    async fn duplicate_increments_mentions() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "likes", "sushi").await.unwrap();
        g.add_relation("alice", "Alice", "likes", "sushi").await.unwrap();

        let rels = g.get_all("alice").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].mentions, 2);
    }

    #[tokio::test]
    async fn single_value_soft_deletes_old() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "lives_in", "Tokyo").await.unwrap();
        g.add_relation("alice", "Alice", "lives_in", "London").await.unwrap();

        let rels = g.get_all("alice").await.unwrap(); // only valid=1
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].destination, "London");
    }

    #[tokio::test]
    async fn multi_value_keeps_both() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "likes", "sushi").await.unwrap();
        g.add_relation("alice", "Alice", "likes", "pizza").await.unwrap();

        let rels = g.get_all("alice").await.unwrap();
        assert_eq!(rels.len(), 2);
    }

    #[tokio::test]
    async fn search_finds_by_keyword() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "works_at", "Google").await.unwrap();
        g.add_relation("alice", "Alice", "likes", "sushi").await.unwrap();

        let results = g.search("alice", "Google").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].destination, "Google");
    }

    #[tokio::test]
    async fn reset_clears_all() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "likes", "sushi").await.unwrap();
        g.reset("alice").await.unwrap();
        assert!(g.get_all("alice").await.unwrap().is_empty());
    }

    #[test]
    fn multi_value_detection() {
        assert!(is_multi_value_relation("likes"));
        assert!(is_multi_value_relation("LOVES"));
        assert!(!is_multi_value_relation("lives_in"));
        assert!(!is_multi_value_relation("works_at"));
        assert!(!is_multi_value_relation("born_in"));
    }

    #[tokio::test]
    async fn user_isolation() {
        let g = test_graph().await;
        g.add_relation("alice", "Alice", "likes", "sushi").await.unwrap();
        g.add_relation("bob", "Bob", "likes", "pizza").await.unwrap();

        let alice_rels = g.get_all("alice").await.unwrap();
        assert_eq!(alice_rels.len(), 1);
        assert_eq!(alice_rels[0].source, "Alice");
    }
}
