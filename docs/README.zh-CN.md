🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

# R-Mem

**以 Rust 研究 [mem0](https://github.com/mem0ai/mem0) 记忆架构的轻量实现。AI agent 的长期记忆。单一可执行文件。无需 Python。**

> 本项目以学习为目的，用 Rust 重新实现 [mem0](https://github.com/mem0ai/mem0) 优雅的记忆架构。完全归功于 mem0 团队的原始设计。这不是替代品，而是以不同语言对其架构的研究。欢迎一起探讨、交流与贡献！

下表反映的是刻意的取舍 — mem0 更丰富的生态系统提供了更多灵活性与集成；R-Mem 有意牺牲这些来换取最小化的部署。

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| 可执行文件 / Runtime | 3.2 MB 静态链接    | Python + pip（丰富生态系统）   |
| 空闲内存 (RSS)     | < 10 MB            | 200 MB+（加载更多功能）        |
| 代码行数           | 1,748              | ~91,500（支持 26+ 种 store）  |
| Vector Store      | 仅 SQLite           | Qdrant、Chroma、Pinecone 等   |
| Graph Store       | 仅 SQLite           | Neo4j / Memgraph             |
| 依赖               | 编译时内含          | pip install mem0ai           |
| LLM 后端          | 任何 OpenAI 兼容端点（Ollama） | OpenAI、Anthropic 及更多 |

---

## 为什么

mem0 是一个设计精良的记忆系统，拥有丰富的 plugin 生态系统。R-Mem 问的是一个更窄的问题：*如果只把核心记忆逻辑用 Rust 重写，并完全以 SQLite 为后端，会怎样？*

结果是同样的三层架构 — vector memory、graph memory、history — 以 1,748 行 Rust 实现。无需外部服务。一个可执行文件。取舍很明确：集成数量远少于 mem0，但运维开销趋近于零。

使用 Claude Code 构建。

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
