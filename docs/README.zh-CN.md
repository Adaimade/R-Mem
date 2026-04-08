🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

# R-Mem

**mem0 的 Rust 实现。AI agent 的长期记忆。单一可执行文件。无需 Python。**

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| 可执行文件 / Runtime | **3.2 MB** 静态链接 | 需要 Python + pip           |
| 空闲内存 (RSS)     | **< 10 MB**        | 200 MB+                      |
| 代码行数           | **1,748**          | ~91,500                      |
| Vector Store      | SQLite（内置）      | Qdrant + 26 种以上            |
| Graph Store       | SQLite（内置）      | Neo4j / Memgraph             |
| 依赖               | 编译时内含          | pip install mem0ai           |
| LLM 后端          | 任何 OpenAI 兼容端点（Ollama） | 仅 OpenAI / Anthropic |

---

## 为什么

mem0 很强大。但它有 91,500 行 Python 代码、需要运行中的向量数据库，而且在做任何事之前就会消耗 200MB+ 的内存。

R-Mem 是同样的三层记忆架构 — vector memory、graph memory、history — 仅用 1,748 行 Rust 实现。SQLite 同时处理向量和图存储。无需外部服务。无需 runtime。一个可执行文件搞定。

完全使用 Claude Code 构建。

> **注意：** 本项目是一项基于 Rust 写法重新实现 AI 记忆系统的研究。核心逻辑与架构参考自 [mem0](https://github.com/mem0ai/mem0)。欢迎一起探讨、交流与贡献！

---

## 工作原理

```
Input text
│
├── Vector Memory（长期事实）
│     ├── LLM 提取事实 → ["Name is Alice", "Works at Google"]
│     ├── Embedding → cosine similarity 搜索（前 5 条现有记忆）
│     ├── Integer ID mapping（防止 LLM UUID 幻觉）
│     ├── LLM 针对每个事实决策：
│     │     ├── ADD       → 新信息
│     │     ├── UPDATE    → 更具体（"likes sports" → "likes tennis"）
│     │     ├── DELETE    → 矛盾（"likes pizza" vs "hates pizza"）
│     │     └── NONE      → 重复，跳过
│     └── 执行 + 写入历史
│
└── Graph Memory（实体关系）
      ├── LLM 提取实体 + 关系
      ├── 冲突检测（soft-delete 旧数据，添加新数据）
      └── 多值 vs 单值关系处理
```

---

## 快速开始

### 前置条件

- Rust 工具链（`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`）
- LLM 后端：[Ollama](https://ollama.com)（本地）或任何 OpenAI 兼容端点

### 构建

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# 可执行文件：target/release/rustmem（3.2 MB）
```

### 配置

创建 `rustmem.toml`：

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

或使用 OpenAI：

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

## 使用方式

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

## 架构

```
src/
├── main.rs        # CLI (clap)
├── config.rs      # TOML + 环境变量配置
├── server.rs      # REST API (axum)
├── memory.rs      # 核心协调器
├── extract.rs     # LLM prompts：事实/实体/关系提取
├── embedding.rs   # OpenAI 兼容 embedding 客户端
├── store.rs       # SQLite vector store（cosine similarity）
└── graph.rs       # SQLite graph store（soft-delete、多值关系）
```

---

## 与 AI Agent 集成

```python
# mem0（之前）
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem（之后 — 通过 HTTP 直接替换）
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 路线图

- [ ] MCP server — 将记忆作为 MCP tools 提供给 Claude / Cursor
- [ ] 批量导入 — 加载现有 mem0 导出数据
- [ ] 多模态 — 图片/音频记忆支持
- [ ] Agent SDK — Rust crate 直接嵌入（无需 HTTP）
- [ ] Dashboard — 轻量级 web UI 用于记忆检查

欢迎社区贡献。开 issue 或 PR 即可。

---

## 许可证

MIT
