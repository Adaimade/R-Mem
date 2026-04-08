🌐 [繁體中文](docs/README.zh-TW.md) · [简体中文](docs/README.zh-CN.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

# R-Mem

**Rust implementation of mem0. Long-term memory for AI agents. Single binary. No Python.**

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| Binary / Runtime  | **3.2 MB** static  | requires Python + pip        |
| Idle Memory (RSS) | **< 10 MB**        | 200 MB+                      |
| Lines of Code     | **1,748**          | ~91,500                      |
| Vector Store      | SQLite (built-in)  | Qdrant + 26 others           |
| Graph Store       | SQLite (built-in)  | Neo4j / Memgraph             |
| Dependencies      | Compiled in        | pip install mem0ai           |
| LLM Backend       | Any OpenAI-compatible (Ollama) | OpenAI / Anthropic only |

---

## Why

mem0 is powerful. It's also 91,500 lines of Python, requires a running vector database, and consumes 200MB+ of RAM before it does anything.

R-Mem is the same three-tier memory architecture — vector memory, graph memory, history — in 1,748 lines of Rust. SQLite handles both vector and graph storage. No external services. No runtime. One binary.

Built entirely with Claude Code.

> **Note:** This project is a research study on reimplementing AI memory systems in Rust. The core logic and architecture are based on [mem0](https://github.com/mem0ai/mem0). Discussions, ideas, and contributions are welcome!

---

## How It Works

```
Input text
│
├── Vector Memory (long-term facts)
│     ├── LLM extracts facts → ["Name is Alice", "Works at Google"]
│     ├── Embedding → cosine similarity search (top-5 existing)
│     ├── Integer ID mapping (prevents LLM UUID hallucination)
│     ├── LLM decides per fact:
│     │     ├── ADD       → new information
│     │     ├── UPDATE    → more specific ("likes sports" → "likes tennis")
│     │     ├── DELETE    → contradiction ("likes pizza" vs "hates pizza")
│     │     └── NONE      → duplicate, skip
│     └── Execute + write history
│
└── Graph Memory (entity relations)
      ├── LLM extracts entities + relations
      ├── Conflict detection (soft-delete old, add new)
      └── Multi-value vs single-value relation handling
```

---

## Quick Start

### Prerequisites

- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- An LLM backend: [Ollama](https://ollama.com) (local) or any OpenAI-compatible endpoint

### Build

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# Binary: target/release/rustmem (3.2 MB)
```

### Configure

Create `rustmem.toml`:

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

Or for OpenAI:

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

---

## Usage

### CLI

```bash
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."
rustmem search -u alice "What does Alice eat?"
rustmem list -u alice
rustmem graph -u alice
rustmem server
```

### REST API

```bash
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

curl http://localhost:8019/memories?user_id=alice
curl -X DELETE http://localhost:8019/memories/{id}
curl http://localhost:8019/memories/{id}/history
```

---

## Architecture

```
src/
├── main.rs        # CLI (clap)
├── config.rs      # TOML + env config
├── server.rs      # REST API (axum)
├── memory.rs      # Core orchestrator
├── extract.rs     # LLM prompts: fact/entity/relation extraction
├── embedding.rs   # OpenAI-compatible embedding client
├── store.rs       # SQLite vector store (cosine similarity)
└── graph.rs       # SQLite graph store (soft-delete, multi-value relations)
```

---

## Integrating with AI Agents

```python
# mem0 (before)
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem (after — drop-in via HTTP)
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## Roadmap

- [ ] MCP server — expose memory as MCP tools for Claude / Cursor
- [ ] Batch import — load existing mem0 exports
- [ ] Multi-modal — image/audio memory support
- [ ] Agent SDK — Rust crate for direct embedding (no HTTP)
- [ ] Dashboard — lightweight web UI for memory inspection

Community contributions welcome. Open an issue or PR.

---

## License

MIT
