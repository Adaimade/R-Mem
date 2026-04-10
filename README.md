<div align="center">

# R-Mem

### Long-term memory for AI agents — in Rust

**A lightweight study of [mem0](https://github.com/mem0ai/mem0)'s memory architecture.**<br>
**Single binary. SQLite-backed. No Python.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/rustmem.svg)](https://crates.io/crates/rustmem)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)
[![Awesome SQLite](https://img.shields.io/badge/Awesome-SQLite-green.svg)](https://github.com/planetopendata/awesome-sqlite)

**3.6 MB binary** · **2,826 lines of Rust** · **< 10 MB RAM** · **SQLite only** · **MCP ready** · **LongMemEval 48.2%**

[Quick Start](#-quick-start) · [How It Works](#-how-it-works) · [Usage](#-usage) · [MCP](#-mcp-server) · [Performance](#-performance) · [Architecture](#-architecture) · [Roadmap](#-roadmap)

🌐 [繁體中文](docs/README.zh-TW.md) · [简体中文](docs/README.zh-CN.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

</div>

> [!NOTE]
> This project reimplements [mem0](https://github.com/mem0ai/mem0)'s elegant memory architecture in Rust as a learning exercise. Full credit to the mem0 team for the original design. This is not a replacement — it's a study of their approach using a different language. Discussions, ideas, and contributions are welcome!

---

## Why R-Mem?

mem0 is a well-designed memory system with a rich plugin ecosystem. R-Mem asks a narrower question: *what if we rewrite just the core memory logic in Rust, backed entirely by SQLite?*

The result is the same three-tier architecture — **vector memory**, **graph memory**, **history** — plus a **tiered archive** system, in **2,826 lines of Rust**. No external services. One binary. The trade-off is clear: far fewer integrations, but near-zero operational overhead.

R-Mem was born out of [RustClaw](https://github.com/Adaimade/RustClaw) — our minimalist Rust AI agent framework. RustClaw needed a memory layer that matched its philosophy: single binary, zero external services. So we studied mem0's architecture and rebuilt it in Rust.

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 Binary</td><td>3.6 MB static</td><td>Python + pip (rich ecosystem)</td></tr>
<tr><td>💾 Idle RSS</td><td>&lt; 10 MB</td><td>200 MB+ (more features loaded)</td></tr>
<tr><td>📝 Code</td><td>2,826 lines</td><td>~91,500 lines (26+ store drivers)</td></tr>
<tr><td>🔍 Vector</td><td>SQLite + FTS5</td><td>Qdrant, Chroma, Pinecone, …</td></tr>
<tr><td>🕸️ Graph</td><td>SQLite only</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>OpenAI, Anthropic, Ollama</td><td>OpenAI, Anthropic, and more</td></tr>
<tr><td>🗄️ Archive</td><td>Tiered memory with fallback</td><td>—</td></tr>
</table>

> mem0's numbers reflect its richer ecosystem — more stores, more integrations, more flexibility. R-Mem intentionally trades that for a minimal footprint.

### What R-Mem adds beyond mem0

| Feature | R-Mem | mem0 |
|---|---|---|
| **Tiered Archive** | Deleted/updated memories preserved + fallback search | Gone when deleted |
| **FTS5 Pre-filter** | Two-stage search: keyword → vector (19x faster) | Vector-only |
| **MCP Server** | Built-in, `rustmem mcp` for Claude Code / Cursor | Not available |
| **Zero-dependency deploy** | Single binary, SQLite, no Docker | Python + pip + vector DB + graph DB |
| **Anthropic native** | Direct Claude API support | Via OpenAI-compatible proxy |
| **Configurable pipeline** | `[memory]` section: thresholds, limits, all tunable | Hardcoded defaults |
| **Memory categories** | Auto-classified: preference, personal, plan, professional, health | Unstructured |

---

## 🔍 How It Works

```
Input text
│
├─ 📦 Vector Memory ──────────────────────────────────
│    │
│    ├─ LLM extracts facts
│    │    → ["Name is Alice", "Works at Google"]
│    │
│    ├─ Embedding → cosine similarity search
│    │    (FTS5 pre-filter + vector ranking)
│    │
│    ├─ Integer ID mapping
│    │    (prevents LLM UUID hallucination)
│    │
│    ├─ LLM decides per fact:
│    │    ├─ ADD       new information
│    │    ├─ UPDATE    more specific → old version archived
│    │    ├─ DELETE    contradiction → old version archived
│    │    └─ NONE      duplicate — skip
│    │
│    └─ Execute actions + write history
│
├─ 🕸️ Graph Memory ──────────────────────────────────
│    │
│    ├─ LLM extracts entities + relations
│    ├─ Conflict detection (soft-delete old, add new)
│    └─ Multi-value vs single-value handling
│
└─ 🗄️ Archive ───────────────────────────────────────
     │
     ├─ Deleted/superseded memories preserved with embeddings
     ├─ Fallback search when active results are weak
     └─ Auto-compaction when archive exceeds threshold
```

---

## 🚀 Quick Start

### Prerequisites

| Requirement | Install |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM backend | [Ollama](https://ollama.com), [OpenAI](https://platform.openai.com), or [Anthropic](https://console.anthropic.com) |

### Install

```bash
cargo install rustmem
```

Or build from source:

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem (3.6 MB)
```

### Configure

Create `rustmem.toml` in the project root:

<table>
<tr>
<td><strong>Ollama (local)</strong></td>
<td><strong>OpenAI</strong></td>
<td><strong>Anthropic</strong></td>
</tr>
<tr>
<td>

```toml
[llm]
provider = "openai"
base_url = "http://127.0.0.1:11434"
model = "qwen2.5:32b"

[embedding]
provider = "openai"
base_url = "http://127.0.0.1:11434"
model = "nomic-embed-text"
```

</td>
<td>

```toml
[llm]
provider = "openai"
api_key = "sk-..."
model = "gpt-4o"

[embedding]
provider = "openai"
api_key = "sk-..."
model = "text-embedding-3-small"
```

</td>
<td>

```toml
[llm]
provider = "anthropic"
api_key = "sk-ant-..."
model = "claude-sonnet-4-6"

[embedding]
provider = "openai"
api_key = "sk-..."
model = "text-embedding-3-small"
```

</td>
</tr>
</table>

> **Note:** Anthropic does not provide embedding models, so `[embedding]` uses OpenAI or Ollama even when `[llm]` uses Anthropic.

> **Security:** R-Mem binds to `127.0.0.1` by default (localhost only). Never put API keys in code — use `rustmem.toml` (gitignored) or environment variables (`RUSTMEM__LLM__API_KEY`).

---

## 📖 Usage

### CLI

```bash
# Add memories
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."

# Semantic search
rustmem search -u alice "What does Alice eat?"

# List all memories for a user
rustmem list -u alice

# Show graph relations
rustmem graph -u alice

# Start REST API server
rustmem server
```

### REST API

Start with `rustmem server`, then:

```bash
# ➕ Add memory
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

# 🔍 Search
curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

# 📋 List all
curl http://localhost:8019/memories?user_id=alice

# 🏷️ Filter by category (preference, personal, plan, professional, health, misc)
curl http://localhost:8019/memories?user_id=alice&category=preference

# 🗑️ Delete
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 History
curl http://localhost:8019/memories/{id}/history

# 🗄️ View archived memories
curl http://localhost:8019/archive?user_id=alice

# 🕸️ View graph relations
curl http://localhost:8019/graph?user_id=alice
```

### Drop-in for AI Agents

```python
# mem0 (before)
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem (after — just switch to HTTP)
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 🔌 MCP Server

R-Mem works as an MCP server — give Claude Code or Cursor long-term memory with one command:

```bash
# Claude Code
claude mcp add rustmem -- /path/to/rustmem mcp

# Cursor (.cursor/mcp.json)
{
  "mcpServers": {
    "rustmem": {
      "command": "/path/to/rustmem",
      "args": ["mcp"]
    }
  }
}
```

**7 tools available:** `add_memory`, `search_memory`, `list_memories`, `get_memory`, `delete_memory`, `get_graph`, `reset_memories`

---

## ⚡ Performance

Benchmarked on Apple Silicon with 10,000 memories (768-dim embeddings):

| Operation | Time | Notes |
|---|---|---|
| **Write** | 36 µs/record | 10K records in 360ms |
| **Brute-force search** | 35.8 ms | Scans all 10K embeddings |
| **FTS5 + vector search** | **1.9 ms** | **19x faster** — pre-filters then re-ranks |
| **Concurrent reads** | 2.4 ms/thread | 10 threads, WAL mode, no blocking |
| **Storage** | 4.2 KB/memory | 10K memories = 40 MB |

Run the benchmark yourself:

```bash
cargo bench --bench store_bench
```

### LongMemEval

[LongMemEval](https://github.com/xiaowu0162/LongMemEval) (ICLR 2025) — 500 questions testing long-term memory across 5 capabilities:

| System | Score | Notes |
|---|---|---|
| agentmemory | 96.2% | RAG (stores raw text) |
| MemLayer | 94.4% | RAG (layered index) |
| Zep | 63.8% | RAG + summary |
| mem0 | ~49% | Fact extraction (gpt-4o) |
| **R-Mem** | **48.2%** | **Fact extraction (gpt-4o-mini)** |

> R-Mem nearly matches mem0 using a 20x cheaper model. The gap vs RAG systems is architectural — R-Mem extracts and deduplicates facts rather than storing raw text, which trades verbatim recall for efficient long-term knowledge management.

---

## 🏗️ Architecture

```
src/
├── main.rs          CLI entry point (clap)
├── config.rs        TOML + env var config
├── server.rs        REST API (axum)
├── mcp.rs           MCP server (rmcp) — 7 tools over stdio
├── memory.rs        Core orchestrator — tiered memory pipeline
├── extract.rs       LLM calls: OpenAI + Anthropic native
├── embedding.rs     OpenAI-compatible embedding client
├── store.rs         SQLite vector store (WAL + FTS5 + archive)
└── graph.rs         SQLite graph store (soft-delete, multi-value)
```

**9 files. 2,826 lines. 3.6 MB binary. Zero external services.**

---

## 🗺️ Roadmap

| Status | Feature | Description |
|---|---|---|
| ✅ | **Published on crates.io** | `cargo install rustmem` — one-line install |
| ✅ | **MCP Server** | `rustmem mcp` — 7 tools over stdio for Claude Code / Cursor |
| ✅ | **Tiered Archive** | Deleted/updated memories preserved + fallback search |
| ✅ | **FTS5 Two-Stage Search** | Keyword pre-filter + vector re-rank — 19x faster |
| ✅ | **Memory Categories** | Auto-classified: preference, personal, plan, professional, health |
| ✅ | **Anthropic Native** | Direct Claude API support (no proxy needed) |
| ✅ | **Agent SDK (lib crate)** | Use `rustmem::{memory, store, graph}` directly in your Rust code |
| ✅ | **LongMemEval Benchmark** | 48.2% with gpt-4o-mini, nearly matching mem0 (~49%) |
| ✅ | **Production Audit** | 11 security/stability fixes, 25 unit tests, cargo bench |
| 🔲 | **Episodic Memory** | Task execution history (tool calls, params, results) |
| 🔲 | **User Preference Model** | Cross-session user style and behavior modeling |
| 🔲 | **Skill Abstraction** | Auto-extract repeated successful patterns into skills |
| 🔲 | **Batch Import** | Load existing mem0 exports |
| 🔲 | **Multi-modal** | Image / audio memory support |
| 🔲 | **Dashboard** | Lightweight web UI for memory inspection |

Community contributions welcome — open an issue or PR.

---

<div align="center">

**MIT License** · v0.3.0

Created by [Ad Huang](https://github.com/Adaimade) with [Claude Code](https://claude.ai)

</div>
