<div align="center">

# R-Mem

### Long-term memory for AI agents — in Rust

**A lightweight study of [mem0](https://github.com/mem0ai/mem0)'s memory architecture.**<br>
**Single binary. SQLite-backed. No Python.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)

[Quick Start](#-quick-start) · [How It Works](#-how-it-works) · [Usage](#-usage) · [Architecture](#-architecture) · [Roadmap](#-roadmap)

🌐 [繁體中文](docs/README.zh-TW.md) · [简体中文](docs/README.zh-CN.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

</div>

> [!NOTE]
> This project reimplements [mem0](https://github.com/mem0ai/mem0)'s elegant memory architecture in Rust as a learning exercise. Full credit to the mem0 team for the original design. This is not a replacement — it's a study of their approach using a different language. Discussions, ideas, and contributions are welcome!

---

## Why R-Mem?

mem0 is a well-designed memory system with a rich plugin ecosystem. R-Mem asks a narrower question: *what if we rewrite just the core memory logic in Rust, backed entirely by SQLite?*

The result is the same three-tier architecture — **vector memory**, **graph memory**, **history** — in **1,748 lines of Rust**. No external services. One binary. The trade-off is clear: far fewer integrations, but near-zero operational overhead.

R-Mem was born out of [RustClaw](https://github.com/Adaimade/RustClaw) — our minimalist Rust AI agent framework. RustClaw needed a memory layer that matched its philosophy: single binary, zero external services. So we studied mem0's architecture and rebuilt it in Rust.

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 Binary</td><td>3.2 MB static</td><td>Python + pip (rich ecosystem)</td></tr>
<tr><td>💾 Idle RSS</td><td>&lt; 10 MB</td><td>200 MB+ (more features loaded)</td></tr>
<tr><td>📝 Code</td><td>1,748 lines</td><td>~91,500 lines (26+ store drivers)</td></tr>
<tr><td>🔍 Vector</td><td>SQLite only</td><td>Qdrant, Chroma, Pinecone, …</td></tr>
<tr><td>🕸️ Graph</td><td>SQLite only</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>Any OpenAI-compatible (Ollama)</td><td>OpenAI, Anthropic, and more</td></tr>
</table>

> mem0's numbers reflect its richer ecosystem — more stores, more integrations, more flexibility. R-Mem intentionally trades that for a minimal footprint.

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
│    ├─ Embedding → cosine similarity search (top-5)
│    │
│    ├─ Integer ID mapping
│    │    (prevents LLM UUID hallucination)
│    │
│    ├─ LLM decides per fact:
│    │    ├─ ADD       new information
│    │    ├─ UPDATE    more specific
│    │    │             "likes sports" → "likes tennis"
│    │    ├─ DELETE    contradiction
│    │    │             "likes pizza" → "hates pizza"
│    │    └─ NONE      duplicate — skip
│    │
│    └─ Execute actions + write history
│
└─ 🕸️ Graph Memory ──────────────────────────────────
     │
     ├─ LLM extracts entities + relations
     ├─ Conflict detection (soft-delete old, add new)
     └─ Multi-value vs single-value handling
```

---

## 🚀 Quick Start

### Prerequisites

| Requirement | Install |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM backend | [Ollama](https://ollama.com) (local) or any OpenAI-compatible API |

### Build & Run

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem (3.2 MB)
```

### Configure

Create `rustmem.toml` in the project root:

<table>
<tr>
<td><strong>Ollama (local)</strong></td>
<td><strong>OpenAI</strong></td>
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
</tr>
</table>

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

# 🗑️ Delete
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 History
curl http://localhost:8019/memories/{id}/history
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

## 🏗️ Architecture

```
src/
├── main.rs          CLI entry point (clap)
├── config.rs        TOML + env var config
├── server.rs        REST API (axum)
├── memory.rs        Core orchestrator — 3-tier memory pipeline
├── extract.rs       LLM prompts: fact / entity / relation extraction
├── embedding.rs     OpenAI-compatible embedding client
├── store.rs         SQLite vector store (cosine similarity)
└── graph.rs         SQLite graph store (soft-delete, multi-value)
```

**8 files. 1,748 lines. Zero external services.**

---

## 🗺️ Roadmap

| Status | Feature | Description |
|---|---|---|
| 🔲 | **MCP Server** | Expose memory as MCP tools for Claude / Cursor |
| 🔲 | **Batch Import** | Load existing mem0 exports |
| 🔲 | **Multi-modal** | Image / audio memory support |
| 🔲 | **Agent SDK** | Rust crate for direct embedding (no HTTP) |
| 🔲 | **Dashboard** | Lightweight web UI for memory inspection |

Community contributions welcome — open an issue or PR.

---

<div align="center">

**MIT License**

Created by [Ad Huang](https://github.com/Adaimade) with [Claude Code](https://claude.ai)

</div>
