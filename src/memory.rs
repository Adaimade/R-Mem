use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::embedding;
use crate::extract::{self, FactAction};
use crate::graph::GraphStore;
use crate::store::{MemoryRecord, MemoryStore, SearchResult};

/// Core memory manager — orchestrates the three-tier memory system.
pub struct MemoryManager {
    config: AppConfig,
    store: MemoryStore,
    graph: GraphStore,
}

/// Result of an add() operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AddResult {
    pub id: String,
    pub text: String,
    pub event: String, // ADD, UPDATE, DELETE, NONE
}

impl MemoryManager {
    pub async fn new(config: &AppConfig) -> Result<Self> {
        let store = MemoryStore::open(&config.store.db_path)?;
        let graph = GraphStore::open(&config.store.db_path)?;
        Ok(Self {
            config: config.clone(),
            store,
            graph,
        })
    }

    // ── ADD: The core memory ingestion flow ──────────────────────────

    /// Add memories from text. This is the main entry point.
    /// 1. Extract facts via LLM
    /// 2. For each fact, vector-search existing memories (top-5)
    /// 3. Map UUIDs to integers for LLM
    /// 4. LLM decides ADD/UPDATE/DELETE/NONE
    /// 5. Execute actions
    /// 6. In parallel: extract entities + relations for graph
    pub async fn add(&self, user_id: &str, text: &str) -> Result<Vec<AddResult>> {
        // Step 1: Extract facts (with categories)
        let categorized_facts = extract::extract_facts(&self.config.llm, text).await?;
        info!(count = categorized_facts.len(), "Extracted facts");

        if categorized_facts.is_empty() {
            return Ok(Vec::new());
        }

        // Extract fact strings for embedding/dedup, keep categories alongside
        let fact_strings: Vec<String> = categorized_facts.iter().map(|cf| cf.fact.clone()).collect();
        let category_map: std::collections::HashMap<String, String> = categorized_facts
            .iter()
            .map(|cf| (cf.fact.clone(), cf.category.clone()))
            .collect();

        // Step 2: Embed all facts in parallel, then search for similar existing memories
        let top_k = self.config.memory.search_top_k;
        let embed_futures: Vec<_> = fact_strings
            .iter()
            .map(|f| embedding::embed(&self.config.embedding, f))
            .collect();
        let fact_embeddings = futures::future::try_join_all(embed_futures).await?;

        let mut all_existing: Vec<(String, String)> = Vec::new(); // (id, text)
        let mut seen_ids = std::collections::HashSet::new();

        for query_emb in &fact_embeddings {
            let similar = self.store.search(user_id, query_emb, top_k).await?;
            for s in similar {
                if seen_ids.insert(s.id.clone()) {
                    all_existing.push((s.id, s.text));
                }
            }
        }

        // Step 3: Integer ID mapping (prevent LLM UUID hallucination)
        let mut uuid_map: Vec<(String, String)> = Vec::new(); // (int_id, real_uuid)
        let existing_for_llm: Vec<(String, String)> = all_existing
            .iter()
            .enumerate()
            .map(|(i, (uuid, text))| {
                uuid_map.push((i.to_string(), uuid.clone()));
                (i.to_string(), text.clone())
            })
            .collect();

        // Step 4: LLM deduplication — decide ADD/UPDATE/DELETE/NONE
        let decisions =
            extract::deduplicate(&self.config.llm, &existing_for_llm, &fact_strings).await?;

        // Step 5: Execute actions
        let mut results = Vec::new();
        for decision in decisions {
            let category = category_map.get(&decision.fact).map(|s| s.as_str()).unwrap_or("misc");
            match decision.action {
                FactAction::Add => {
                    let id = Uuid::new_v4().to_string();
                    let emb = embedding::embed(&self.config.embedding, &decision.fact).await?;
                    self.store.add(&id, user_id, &decision.fact, category, &emb).await?;
                    info!(id = %id, "Memory ADD: {}", decision.fact);
                    results.push(AddResult {
                        id,
                        text: decision.fact,
                        event: "ADD".to_string(),
                    });
                }
                FactAction::Update => {
                    if let Some(ref int_id) = decision.existing_id {
                        // Map integer ID back to real UUID
                        let real_id = uuid_map
                            .iter()
                            .find(|(k, _)| k == int_id)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| int_id.clone());

                        let emb =
                            embedding::embed(&self.config.embedding, &decision.fact).await?;
                        self.store.update(&real_id, &decision.fact, &emb).await?;
                        info!(id = %real_id, "Memory UPDATE: {}", decision.fact);
                        results.push(AddResult {
                            id: real_id,
                            text: decision.fact,
                            event: "UPDATE".to_string(),
                        });
                    }
                }
                FactAction::Delete => {
                    if let Some(ref int_id) = decision.existing_id {
                        let real_id = uuid_map
                            .iter()
                            .find(|(k, _)| k == int_id)
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| int_id.clone());

                        self.store.delete(&real_id).await?;
                        info!(id = %real_id, "Memory DELETE");
                        results.push(AddResult {
                            id: real_id,
                            text: decision.fact,
                            event: "DELETE".to_string(),
                        });
                    }
                }
                FactAction::None => {
                    // No action needed
                }
            }
        }

        // Step 6: Graph — extract entities and relations (concurrent with above in mem0,
        // we do it sequentially for simplicity but could tokio::spawn)
        if let Err(e) = self.add_to_graph(user_id, text).await {
            tracing::warn!(%e, "Graph extraction failed (non-fatal)");
        }

        Ok(results)
    }

    // ── SEARCH ───────────────────────────────────────────────────────

    pub async fn search(
        &self,
        user_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let query_emb = embedding::embed(&self.config.embedding, query).await?;
        // Two-stage search: FTS5 pre-filter → vector ranking
        let mut results = self.store.search_with_fts(user_id, query, &query_emb, limit).await?;

        // Tag active results
        for r in &mut results {
            if r.source.is_none() {
                r.source = Some("active".to_string());
            }
        }

        // Fallback to archive if active results are weak
        let best_active_score = results.first().map(|r| r.score).unwrap_or(0.0);
        if best_active_score < self.config.memory.archive_fallback_threshold {
            let archive_results = self.store.search_archive(user_id, &query_emb, limit).await?;
            if !archive_results.is_empty() {
                info!(count = archive_results.len(), "Archive fallback triggered");
                results.extend(archive_results);
            }
        }

        // Also search graph for relations
        let relations = self.graph.search_with_limit(user_id, query, self.config.memory.graph_search_limit).await?;
        if !relations.is_empty() {
            for rel in relations {
                let text = format!("{} {} {}", rel.source, rel.relation, rel.destination);
                results.push(SearchResult {
                    id: format!("graph:{}", rel.source),
                    text,
                    score: self.config.memory.graph_match_score,
                    user_id: user_id.to_string(),
                    source: Some("graph".to_string()),
                    created_at: None,
                });
            }
        }

        // Sort by score descending, deduplicate by text
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.dedup_by(|a, b| a.text == b.text);
        results.truncate(limit);

        Ok(results)
    }

    // ── CRUD ─────────────────────────────────────────────────────────

    pub async fn get(&self, id: &str) -> Result<Option<MemoryRecord>> {
        self.store.get(id).await
    }

    pub async fn get_all(&self, user_id: &str) -> Result<Vec<MemoryRecord>> {
        self.store.get_all(user_id).await
    }

    pub async fn get_by_category(&self, user_id: &str, category: &str) -> Result<Vec<MemoryRecord>> {
        self.store.get_by_category(user_id, category).await
    }

    pub async fn update(&self, id: &str, text: &str) -> Result<()> {
        let emb = embedding::embed(&self.config.embedding, text).await?;
        self.store.update(id, text, &emb).await
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        self.store.delete(id).await
    }

    pub async fn reset(&self, user_id: &str) -> Result<u64> {
        let count = self.store.reset(user_id).await?;
        self.graph.reset(user_id).await?;
        Ok(count)
    }

    pub async fn history(&self, id: &str) -> Result<Vec<serde_json::Value>> {
        self.store.history(id).await
    }

    /// Get archived memories for a user.
    pub async fn get_archive(&self, user_id: &str) -> Result<Vec<crate::store::ArchivedRecord>> {
        self.store.get_archive(user_id).await
    }

    /// Get archive entry count for a user.
    pub async fn archive_count(&self, user_id: &str) -> Result<usize> {
        self.store.archive_count(user_id).await
    }

    /// Compact archive: keep only the most recent N entries.
    pub async fn compact_archive(&self, user_id: &str) -> Result<usize> {
        let max = self.config.memory.archive_max_entries;
        self.store.compact_archive(user_id, max).await
    }

    /// Get all graph relations for a user.
    pub async fn get_graph(&self, user_id: &str) -> Result<Vec<crate::graph::Relation>> {
        self.graph.get_all(user_id).await
    }

    // ── Graph Memory ─────────────────────────────────────────────────

    /// Extract entities and relations from text and store in graph.
    async fn add_to_graph(&self, user_id: &str, text: &str) -> Result<()> {
        // Step 1: Extract entities (with self-reference resolution)
        let entities = extract::extract_entities(&self.config.llm, text, user_id).await?;

        if entities.is_empty() {
            return Ok(());
        }

        // Step 2: Extract relations between entities
        let relations =
            extract::extract_relations(&self.config.llm, text, &entities).await?;

        info!(
            entities = entities.len(),
            relations = relations.len(),
            "Graph extraction"
        );

        // Step 3: Store in graph (with conflict resolution)
        for rel in &relations {
            self.graph
                .add_relation(user_id, &rel.source, &rel.relation, &rel.destination)
                .await?;
        }

        Ok(())
    }
}
