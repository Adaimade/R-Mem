

<div align="center">

# R-Mem

### Memoria a largo plazo para agentes de IA — en Rust

**Un estudio ligero de la arquitectura de memoria de [mem0](https://github.com/mem0ai/mem0).**<br>
**Un solo binario. Respaldado por SQLite. Sin Python.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/rustmem.svg)](https://crates.io/crates/rustmem)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)
[![Awesome SQLite](https://img.shields.io/badge/Awesome-SQLite-green.svg)](https://github.com/planetopendata/awesome-sqlite)

**Binario de 3,6 MB** · **2.826 líneas de Rust** · **< 10 MB de RAM** · **Solo SQLite** · **Listo para MCP** · **LongMemEval 48,2%**

[Inicio Rápido](#-quick-start) · [Guía de Integración](#-integration-guide) · [¿Cómo Funciona?](#-how-it-works) · [Uso](#-usage) · [MCP](#-mcp-server) · [Rendimiento](#-performance) · [Arquitectura](#-architecture) · [Hoja de Ruta](#-roadmap)

🌐 [繁體中文](docs/README.zh-TW.md) · [简体中文](docs/README.zh-CN.md) · [日本語](docs/README.ja.md) · [한국어](docs/README.ko.md)

</div>

> [!NOTE]
> Este proyecto reimplanta la elegante arquitectura de memoria de [mem0](https://github.com/mem0ai/mem0) en Rust como ejercicio de aprendizaje. Todo el crédito al equipo de mem0 por el diseño original. Esto no es un reemplazo: es un estudio de su enfoque utilizando un lenguaje diferente. ¡Discusiones, ideas y contribuciones son bienvenidas!

---

## ¿Por qué R-Mem?

mem0 es un sistema de memoria bien diseñado con un rico ecosistema de complementos. R-Mem plantea una pregunta más específica: *¿qué pasaría si reescribiéramos solo la lógica central de memoria en Rust, respaldada enteramente por SQLite?*

El resultado es la misma arquitectura de tres niveles — **memoria vectorial**, **memoria gráfica**, **historial** — más un sistema de **archivado por niveles**, en **2.826 líneas de Rust**. Sin servicios externos. Un solo binario. El intercambio es claro: muchas menos integraciones, pero una carga operativa casi nula.

R-Mem nació de [RustClaw](https://github.com/Adaimade/RustClaw), nuestro marco de trabajo minimalista para agentes de IA en Rust. RustClaw necesitaba una capa de memoria que coincidiera con su filosofía: binario único, cero servicios externos. Así que estudiamos la arquitectura de mem0 y la reconstruimos en Rust.

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 Binario</td><td>Estático de 3,6 MB</td><td>Python + pip (ecosistema rico)</td></tr>
<tr><td>💾 RSS en reposo</td><td>&lt; 10 MB</td><td>200 MB+ (más funciones cargadas)</td></tr>
<tr><td>📝 Código</td><td>2.826 líneas</td><td>~91.500 líneas (26+ controladores de almacenamiento)</td></tr>
<tr><td>🔍 Vector</td><td>SQLite + FTS5</td><td>Qdrant, Chroma, Pinecone, …</td></tr>
<tr><td>🕸️ Gráfica</td><td>Solo SQLite</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>OpenAI, Anthropic, Ollama</td><td>OpenAI, Anthropic y más</td></tr>
<tr><td>🗄️ Archivo</td><td>Memoria por niveles con recuperación</td><td>—</td></tr>
</table>

> Las cifras de mem0 reflejan su ecosistema más rico: más almacenes, más integraciones, más flexibilidad. R-Mem cambia intencionalmente eso por una huella mínima.

### Lo que R-Mem añade más allá de mem0

| Característica | R-Mem | mem0 |
|---|---|---|
| **Archivo por niveles** | Memorias eliminadas/actualizadas preservadas + búsqueda de recuperación | Se pierden al eliminarlas |
| **Prefiltro FTS5** | Búsqueda en dos etapas: palabra clave → vector (19x más rápido) | Solo vector |
| **Servidor MCP** | Incluido, `rustmem mcp` para Claude Code / Cursor | No disponible |
| **Despliegue sin dependencias** | Binario único, SQLite, sin Docker | Python + pip + BD vectorial + BD gráfica |
| **Anthropic Nativo** | Soporte directo a la API de Claude | A través de proxy compatible con OpenAI |
| **Pipeline configurable** | Sección `[memory]`: umbrales, límites, todo ajustable | Valores predeterminados codificados |
| **Categorías de memoria** | Clasificación automática: preferencia, personal, plan, profesional, salud | No estructuradas |

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

| Requisito | Instalación |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Backend de LLM | [Ollama](https://ollama.com), [OpenAI](https://platform.openai.com), o [Anthropic](https://console.anthropic.com) |

### Install

```bash
cargo install rustmem
```

O compilar desde el código fuente:

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem (3.6 MB)
```

### Configure

Crea `rustmem.toml` en la raíz del proyecto:

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

> **Nota:** Anthropic no proporciona modelos de incrustación, por lo que `[embedding]` utiliza OpenAI u Ollama incluso cuando `[llm]` usa Anthropic.

> **Seguridad:** R-Mem se vincula a `127.0.0.1` de forma predeterminada (solo localhost). Nunca pongas claves API en el código: usa `rustmem.toml` (ignorado por git) o variables de entorno (`RUSTMEM__LLM__API_KEY`).

---

## 🔗 Integration Guide

### ⚠️ Building a MemoryManager is not enough

El error de integración más común: inicializas `MemoryManager`, pero nunca llamas a `add()` o `search()` en tu bucle de conversación. El sistema de memoria existe pero nunca se usa: nada de lo que diga el usuario se recuerda.

### The correct conversation loop

Cada turno debe incluir dos operaciones de memoria:

1. **Antes de llamar al LLM — RECOLECCIÓN** (buscar memorias relevantes)
2. **Después de llamar al LLM — APRENDIZAJE** (extraer y almacenar nuevos hechos)

```
loop {
    user_message = receive()

    // 1. RECALL — before calling the LLM
    memories = rmem.search(user_id, user_message, limit=10)
    context = format_as_context(memories)

    // 2. Call LLM with memory context
    response = llm.chat(system_prompt + context + user_message)

    // 3. LEARN — after responding
    rmem.add(user_id, user_message)

    send(response)
}
```

### Memory context format

Formatea los resultados de `search()` como un contexto que el LLM pueda entender:

```
[Memory]
Known facts about this user:
- User's name is Alice
- User prefers dark mode
- User is working on a Rust project
```

Coloca esto en el indicador del sistema o antes del mensaje del usuario para que el LLM pueda hacer referencia a él.

### Multi-scope pattern

Si tu aplicación sirve múltiples canales (p. ej., Telegram + Discord), usa tres capas de alcance:

| Alcance | Propósito | ID de ejemplo |
|---|---|---|
| local | Conversación/grupo único | `telegram:group_123` |
| user | Memoria personal entre canales | `user:456` |
| global | Compartida entre todos los usuarios | `global:system` |

Fusiona los resultados en el momento de la recolección:

```
local_results  = search("telegram:group_123", query)
user_results   = search("user:456", query)
global_results = search("global:system", query)
all = deduplicate(local + user + global)
```

### Common mistakes

- ❌ Inicializar MemoryManager pero nunca llamar a `search()` / `add()` en el bucle
- ❌ Solo APRENDER sin RECOLECCIÓN (memorias almacenadas pero nunca recuperadas)
- ❌ Solo RECOLECTAR sin APRENDER (lee memorias antiguas pero nunca aprende nuevas)
- ❌ Colocar `add()` antes de la llamada al LLM (el mensaje actual se trata como un hecho conocido)

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

Inicia con `rustmem server`, luego:

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

R-Mem funciona como un servidor MCP: otorga memoria a largo plazo a Claude Code o Cursor con un solo comando:

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

**7 herramientas disponibles:** `add_memory`, `search_memory`, `list_memories`, `get_memory`, `delete_memory`, `get_graph`, `reset_memories`

---

## ⚡ Performance

Evaluado en Apple Silicon con 10.000 memorias (incrustaciones de 768 dimensiones):

| Operación | Tiempo | Notas |
|---|---|---|
| **Escritura** | 36 µs/registro | 10K registros en 360ms |
| **Búsqueda por fuerza bruta** | 35,8 ms | Escanea las 10K incrustaciones |
| **Búsqueda FTS5 + vector** | **1,9 ms** | **19x más rápido** — prefiltra y luego reclasifica |
| **Lecturas concurrentes** | 2,4 ms/hilo | 10 hilos, modo WAL, sin bloqueo |
| **Almacenamiento** | 4,2 KB/memoria | 10K memorias = 40 MB |

Ejecuta el benchmark tú mismo:

```bash
cargo bench --bench store_bench
```

### LongMemEval

[LongMemEval](https://github.com/xiaowu0162/LongMemEval) (ICLR 2025): 500 preguntas que prueban la memoria a largo plazo en 5 capacidades:

| Sistema | Puntuación | Notas |
|---|---|---|
| agentmemory | 96,2% | RAG (almacena texto en bruto) |
| MemLayer | 94,4% | RAG (índice en capas) |
| Zep | 63,8% | RAG + resumen |
| mem0 | ~49% | Extracción de hechos (gpt-4o) |
| **R-Mem** | **48,2%** | **Extracción de hechos (gpt-4o-mini)** |

> R-Mem casi iguala a mem0 usando un modelo 20x más barato. La diferencia con los sistemas RAG es arquitectónica: R-Mem extrae y deduplica hechos en lugar de almacenar texto en bruto, lo que cambia el recuerdo literal por una gestión eficiente del conocimiento a largo plazo.

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

**9 archivos. 2.826 líneas. Binario de 3,6 MB. Cero servicios externos.**

---

## 🗺️ Roadmap

| Estado | Característica | Descripción |
|---|---|---|
| ✅ | **Publicado en crates.io** | `cargo install rustmem` — instalación en una línea |
| ✅ | **Servidor MCP** | `rustmem mcp` — 7 herramientas sobre stdio para Claude Code / Cursor |
| ✅ | **Archivo por niveles** | Memorias eliminadas/actualizadas preservadas + búsqueda de recuperación |
| ✅ | **Búsqueda en dos etapas FTS5** | Prefiltro de palabras clave + reclasificación vectorial — 19x más rápido |
| ✅ | **Categorías de memoria** | Clasificación automática: preferencia, personal, plan, profesional, salud |
| ✅ | **Anthropic Nativo** | Soporte directo a la API de Claude (sin proxy necesario) |
| ✅ | **SDK para Agentes (lib crate)** | Usa `rustmem::{memory, store, graph}` directamente en tu código Rust |
| ✅ | **Benchmark LongMemEval** | 48,2% con gpt-4o-mini, casi igualando a mem0 (~49%) |
| ✅ | **Auditoría de Producción** | 11 correcciones de seguridad/estabilidad, 25 pruebas unitarias, cargo bench |
| 🔲 | **Memoria Episódica** | Historial de ejecución de tareas (llamadas a herramientas, parámetros, resultados) |
| 🔲 | **Modelo de Preferencia de Usuario** | Modelado de estilo y comportamiento del usuario entre sesiones |
| 🔲 | **Abstracción de Habilidades** | Extracción automática de patrones exitosos repetidos en habilidades |
| 🔲 | **Importación por Lotes** | Cargar exportaciones existentes de mem0 |
| 🔲 | **Multimodal** | Soporte para memoria de imagen / audio |
| 🔲 | **Panel de Control** | Interfaz web ligera para inspección de memoria |

R-Mem v0.3.0 está completo en funciones como proyecto de aprendizaje. La arquitectura central es estable y probada para producción. Las contribuciones de la comunidad, bifurcaciones y exploraciones son bienvenidas: abre un issue o PR.

---

<div align="center">

**MIT License** · v0.3.0

Creado por [Ad Huang](https://github.com/Adaimade) con [Claude Code](https://claude.ai)

</div>
