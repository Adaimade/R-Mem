🌐 [繁體中文](docs/README.zh-TW.md) · [简体中文](docs/README.zh-CN.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

# R-Mem

**A lightweight Rust study of [mem0](https://github.com/mem0ai/mem0)'s memory architecture. Long-term memory for AI agents. Single binary. No Python.**

> This project reimplements [mem0](https://github.com/mem0ai/mem0)'s elegant memory architecture in Rust as a learning exercise. Full credit to the mem0 team for the original design. This is not a replacement — it's a study of their approach using a different language. Discussions, ideas, and contributions are welcome!

The table below reflects deliberate trade-offs — mem0's richer ecosystem offers far more flexibility and integrations; R-Mem intentionally sacrifices that for minimal footprint.

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| Binary / Runtime  | 3.2 MB static      | Python + pip (rich ecosystem)|
| Idle Memory (RSS) | < 10 MB            | 200 MB+ (more features loaded)|
| Lines of Code     | 1,748              | ~91,500 (supports 26+ stores)|
| Vector Store      | SQLite only         | Qdrant, Chroma, Pinecone, etc.|
| Graph Store       | SQLite only         | Neo4j / Memgraph             |
| Dependencies      | Compiled in         | pip install mem0ai           |
| LLM Backend       | Any OpenAI-compatible (Ollama) | OpenAI, Anthropic, and more |

---

## Why

mem0 is a well-designed memory system with a rich plugin ecosystem. R-Mem asks a narrower question: *what if we rewrite just the core memory logic in Rust, backed entirely by SQLite?*

The result is the same three-tier architecture — vector memory, graph memory, history — in 1,748 lines of Rust. No external services. One binary. The trade-off is clear: far fewer integrations, but near-zero operational overhead.

Built with Claude Code.

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
