# R-Mem

**A lightweight Rust alternative to [mem0](https://github.com/mem0ai/mem0).** Long-term memory for AI agents. Single binary. No runtime.

|                   | **R-Mem**          | **mem0**                |
|-------------------|--------------------|-------------------------|
| Binary / Runtime  | **3.2 MB** static  | requires Python + pip   |
| Lines of Code     | **1,748**          | ~91,500                 |
| Vector Store      | SQLite (built-in)  | Qdrant + 26 others      |
| Graph Store       | SQLite (built-in)  | Neo4j / Memgraph        |
| Dependencies      | Compiled in        | pip install mem0ai      |

## How It Works

Three-tier memory system (same as mem0):

```
Conversation in
    │
    ├── Long-term Memory (Vector)
    │     ├── LLM extracts facts: ["Name is John", "Works at Google"]
    │     ├── Vector search finds similar existing memories (top-5)
    │     ├── Integer ID mapping (prevents LLM UUID hallucination)
    │     ├── LLM decides: ADD / UPDATE / DELETE / NONE
    │     │     ├── Contradiction → DELETE old ("likes pizza" vs "hates pizza")
    │     │     ├── More specific → UPDATE ("likes sports" → "likes tennis with friends")
    │     │     ├── Same meaning → NONE (skip)
    │     │     └── New topic → ADD
    │     └── Execute actions + write history
    │
    └── Graph Memory (Relations)
          ├── LLM extracts entities ("I" → user_id)
          ├── LLM extracts relations (source, relation, destination)
          ├── Conflict detection (soft-delete old, add new)
          └── Multi-value vs single-value relation handling
```

## Quick Start

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
```

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

## Usage

### CLI

```bash
# Add a memory
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."

# Search memories
rustmem search -u alice "What does Alice eat?"

# List all memories
rustmem list -u alice

# Show graph relations
rustmem graph -u alice

# Start API server
rustmem server
```

### REST API

```bash
# Add memory
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

# Search
curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

# Get all
curl http://localhost:8019/memories?user_id=alice

# Delete
curl -X DELETE http://localhost:8019/memories/{id}

# History
curl http://localhost:8019/memories/{id}/history
```

## Architecture

```
src/
├── main.rs        # CLI (clap)
├── config.rs      # TOML + env config
├── server.rs      # REST API (axum)
├── memory.rs      # Core orchestrator (three-tier memory logic)
├── extract.rs     # LLM prompts: fact extraction, dedup, entity/relation extraction
├── embedding.rs   # OpenAI-compatible embedding API
├── store.rs       # SQLite vector store (cosine similarity)
└── graph.rs       # SQLite graph store (soft-delete, multi-value relations)
```

## License

MIT
