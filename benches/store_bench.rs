use std::time::Instant;

// Import from the main crate
// We'll use direct SQLite operations to test without needing LLM

fn main() {
    println!("=== R-Mem Store Performance Benchmark ===\n");

    let db_path = "/tmp/rustmem_bench.db";
    let _ = std::fs::remove_file(db_path);

    // Setup
    let conn = rusqlite::Connection::open(db_path).unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;").unwrap();
    conn.busy_timeout(std::time::Duration::from_secs(5)).unwrap();

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
         CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
             text, content='memories', content_rowid='rowid'
         );",
    ).unwrap();

    // Generate fake embeddings (768 dims like nomic-embed-text)
    let dims = 768;
    let fake_emb = |seed: f32| -> Vec<u8> {
        (0..dims)
            .map(|i| (seed + i as f32 * 0.001).sin())
            .flat_map(|f| f.to_le_bytes())
            .collect()
    };

    // ── Benchmark 1: Write 10,000 memories ──────────────────────────

    let count = 10_000;
    let start = Instant::now();
    {
        let tx = conn.unchecked_transaction().unwrap();
        for i in 0..count {
            let id = format!("mem_{i:05}");
            let text = format!("This is memory number {i} about topic {} with details {}", i % 50, i * 7);
            let emb = fake_emb(i as f32);
            tx.execute(
                "INSERT INTO memories (id, user_id, text, embedding) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, "bench_user", text, emb],
            ).unwrap();
            // FTS sync
            tx.execute(
                "INSERT INTO memories_fts(rowid, text) SELECT rowid, text FROM memories WHERE id = ?1",
                [&id],
            ).unwrap();
        }
        tx.commit().unwrap();
    }
    let write_dur = start.elapsed();
    println!("📝 Write {count} memories (768-dim embeddings)");
    println!("   Total: {:?}", write_dur);
    println!("   Per write: {:.2}µs", write_dur.as_micros() as f64 / count as f64);
    println!();

    // ── Benchmark 2: Brute-force vector search ──────────────────────

    let query_emb: Vec<f32> = (0..dims).map(|i| (42.0_f32 + i as f32 * 0.001).sin()).collect();

    let start = Instant::now();
    let brute_results = {
        let mut stmt = conn.prepare(
            "SELECT id, text, embedding FROM memories WHERE user_id = ?1",
        ).unwrap();

        let mut results: Vec<(String, f32)> = stmt
            .query_map(["bench_user"], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(2)?;
                Ok((id, blob))
            }).unwrap()
            .filter_map(|r| r.ok())
            .map(|(id, blob)| {
                let emb: Vec<f32> = blob.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let score = cosine_sim(&query_emb, &emb);
                (id, score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(20);
        results
    };
    let brute_dur = start.elapsed();
    println!("🔍 Brute-force search ({count} memories, {dims}-dim)");
    println!("   Time: {:?}", brute_dur);
    println!("   Top result: {} (score: {:.4})", brute_results[0].0, brute_results[0].1);
    println!();

    // ── Benchmark 3: FTS5 pre-filter + vector re-rank ───────────────

    let fts_query = "memory topic details";
    let start = Instant::now();
    let fts_results = {
        // Stage 1: FTS5 pre-filter
        let mut fts_stmt = conn.prepare(
            "SELECT m.id, m.text, m.embedding FROM memories m
             JOIN memories_fts f ON m.rowid = f.rowid
             WHERE m.user_id = ?1 AND memories_fts MATCH ?2
             LIMIT 60",
        ).unwrap();

        let candidates: Vec<(String, Vec<u8>)> = fts_stmt
            .query_map(rusqlite::params!["bench_user", fts_query], |row| {
                let id: String = row.get(0)?;
                let blob: Vec<u8> = row.get(2)?;
                Ok((id, blob))
            }).unwrap()
            .filter_map(|r| r.ok())
            .collect();

        let candidate_count = candidates.len();

        // Stage 2: Vector re-rank
        let mut results: Vec<(String, f32)> = candidates
            .into_iter()
            .map(|(id, blob)| {
                let emb: Vec<f32> = blob.chunks_exact(4)
                    .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let score = cosine_sim(&query_emb, &emb);
                (id, score)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(20);
        (results, candidate_count)
    };
    let fts_dur = start.elapsed();
    let (fts_top, fts_candidate_count) = fts_results;
    println!("⚡ FTS5 pre-filter + vector re-rank");
    println!("   FTS candidates: {fts_candidate_count} / {count}");
    println!("   Time: {:?}", fts_dur);
    if !fts_top.is_empty() {
        println!("   Top result: {} (score: {:.4})", fts_top[0].0, fts_top[0].1);
    }
    let speedup = brute_dur.as_micros() as f64 / fts_dur.as_micros().max(1) as f64;
    println!("   Speedup: {speedup:.1}x faster than brute-force");
    println!();

    // ── Benchmark 4: Concurrent reads (WAL test) ────────────────────

    let start = Instant::now();
    let threads: Vec<_> = (0..10)
        .map(|t| {
            let path = db_path.to_string();
            let q_emb = query_emb.clone();
            std::thread::spawn(move || {
                let conn = rusqlite::Connection::open(&path).unwrap();
                conn.execute_batch("PRAGMA journal_mode=WAL;").unwrap();
                let mut stmt = conn.prepare(
                    "SELECT id, embedding FROM memories WHERE user_id = ?1 LIMIT 1000",
                ).unwrap();
                let results: Vec<(String, f32)> = stmt
                    .query_map(["bench_user"], |row| {
                        let id: String = row.get(0)?;
                        let blob: Vec<u8> = row.get(1)?;
                        Ok((id, blob))
                    }).unwrap()
                    .filter_map(|r| r.ok())
                    .map(|(id, blob)| {
                        let emb: Vec<f32> = blob.chunks_exact(4)
                            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                            .collect();
                        (id, cosine_sim(&q_emb, &emb))
                    })
                    .collect();
                (t, results.len())
            })
        })
        .collect();

    let mut total_results = 0;
    for t in threads {
        let (tid, count) = t.join().unwrap();
        total_results += count;
    }
    let concurrent_dur = start.elapsed();
    println!("🔄 Concurrent reads (10 threads × 1000 memories, WAL mode)");
    println!("   Total results: {total_results}");
    println!("   Time: {:?}", concurrent_dur);
    println!("   Per thread: {:.2}ms", concurrent_dur.as_millis() as f64 / 10.0);
    println!();

    // ── Benchmark 5: DB file size ───────────────────────────────────

    let metadata = std::fs::metadata(db_path).unwrap();
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
    println!("💾 Storage");
    println!("   {count} memories × {dims}-dim embeddings");
    println!("   DB size: {size_mb:.1} MB");
    println!("   Per memory: {:.0} bytes", metadata.len() as f64 / count as f64);
    println!();

    // Summary
    println!("=== Summary ===");
    println!("   Write:        {:.2}ms / {count} records", write_dur.as_millis());
    println!("   Brute search: {:?} / {count} records", brute_dur);
    println!("   FTS+vector:   {:?} / {fts_candidate_count} candidates → {speedup:.1}x faster", fts_dur);
    println!("   Concurrent:   {:?} / 10 threads", concurrent_dur);
    println!("   Storage:      {size_mb:.1} MB / {count} records");

    // Cleanup
    let _ = std::fs::remove_file(db_path);
    let _ = std::fs::remove_file(format!("{db_path}-wal"));
    let _ = std::fs::remove_file(format!("{db_path}-shm"));
}

fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { return 0.0; }
    dot / (mag_a * mag_b)
}
