<div align="center">

# R-Mem

### AI Agent 的长期记忆 — 以 Rust 实现

**以 Rust 研究 [mem0](https://github.com/mem0ai/mem0) 记忆架构的轻量实现。**<br>
**单一可执行文件。SQLite 为后端。无需 Python。**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)

[快速开始](#-快速开始) · [工作原理](#-工作原理) · [使用方式](#-使用方式) · [架构](#️-架构) · [路线图](#️-路线图)

🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

> [!NOTE]
> 本项目以学习为目的，用 Rust 重新实现 [mem0](https://github.com/mem0ai/mem0) 优雅的记忆架构。完全归功于 mem0 团队的原始设计。这不是替代品，而是以不同语言对其架构的研究。欢迎一起探讨、交流与贡献！

---

## 为什么选择 R-Mem？

mem0 是一个设计精良的记忆系统，拥有丰富的 plugin 生态系统。R-Mem 问的是一个更窄的问题：*如果只把核心记忆逻辑用 Rust 重写，并完全以 SQLite 为后端，会怎样？*

结果是同样的三层架构 — **vector memory**、**graph memory**、**history** — 以 **1,748 行 Rust** 实现。无需外部服务。一个可执行文件。取舍很明确：集成数量远少于 mem0，但运维开销趋近于零。

R-Mem 诞生自 [RustClaw](https://github.com/Adaimade/RustClaw) — 我们极简风格的 Rust AI agent 框架。RustClaw 需要一个符合其理念的记忆层：单一可执行文件、零外部服务。因此我们研究了 mem0 的架构，并以 Rust 重新实现。

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 可执行文件</td><td>3.2 MB 静态链接</td><td>Python + pip（丰富生态系统）</td></tr>
<tr><td>💾 空闲 RSS</td><td>&lt; 10 MB</td><td>200 MB+（加载更多功能）</td></tr>
<tr><td>📝 代码</td><td>1,748 行</td><td>~91,500 行（26+ 种 store driver）</td></tr>
<tr><td>🔍 Vector</td><td>仅 SQLite</td><td>Qdrant、Chroma、Pinecone…</td></tr>
<tr><td>🕸️ Graph</td><td>仅 SQLite</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>任何 OpenAI 兼容端点（Ollama）</td><td>OpenAI、Anthropic 及更多</td></tr>
</table>

> mem0 的数字反映的是它更丰富的生态系统 — 更多 store、更多集成、更多灵活性。R-Mem 有意牺牲这些来换取最小化的部署。

---

## 🔍 工作原理

```
Input text
│
├─ 📦 Vector Memory ──────────────────────────────────
│    │
│    ├─ LLM 提取事实
│    │    → ["Name is Alice", "Works at Google"]
│    │
│    ├─ Embedding → cosine similarity 搜索（前 5 条）
│    │
│    ├─ Integer ID mapping
│    │    （防止 LLM UUID 幻觉）
│    │
│    ├─ LLM 针对每个事实决策：
│    │    ├─ ADD       新信息
│    │    ├─ UPDATE    更具体
│    │    │             "likes sports" → "likes tennis"
│    │    ├─ DELETE    矛盾
│    │    │             "likes pizza" → "hates pizza"
│    │    └─ NONE      重复 — 跳过
│    │
│    └─ 执行动作 + 写入历史
│
└─ 🕸️ Graph Memory ──────────────────────────────────
     │
     ├─ LLM 提取实体 + 关系
     ├─ 冲突检测（soft-delete 旧数据，添加新数据）
     └─ 多值 vs 单值关系处理
```

---

## 🚀 快速开始

### 前置条件

| 需求 | 安装方式 |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM 后端 | [Ollama](https://ollama.com)（本地）或任何 OpenAI 兼容 API |

### 构建与运行

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem（3.2 MB）
```

### 配置

在项目根目录创建 `rustmem.toml`：

<table>
<tr>
<td><strong>Ollama（本地）</strong></td>
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

## 📖 使用方式

### CLI

```bash
# 添加记忆
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."

# 语义搜索
rustmem search -u alice "What does Alice eat?"

# 列出用户所有记忆
rustmem list -u alice

# 显示图谱关系
rustmem graph -u alice

# 启动 REST API 服务器
rustmem server
```

### REST API

先启动 `rustmem server`，然后：

```bash
# ➕ 添加记忆
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

# 🔍 搜索
curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

# 📋 列出全部
curl http://localhost:8019/memories?user_id=alice

# 🗑️ 删除
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 历史记录
curl http://localhost:8019/memories/{id}/history
```

### AI Agent 直接替换

```python
# mem0（之前）
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem（之后 — 只需改用 HTTP）
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 🏗️ 架构

```
src/
├── main.rs          CLI 入口（clap）
├── config.rs        TOML + 环境变量配置
├── server.rs        REST API（axum）
├── memory.rs        核心协调器 — 三层记忆管线
├── extract.rs       LLM prompts：事实/实体/关系提取
├── embedding.rs     OpenAI 兼容 embedding 客户端
├── store.rs         SQLite vector store（cosine similarity）
└── graph.rs         SQLite graph store（soft-delete、多值关系）
```

**8 个文件。1,748 行。零外部服务。**

---

## 🗺️ 路线图

| 状态 | 功能 | 说明 |
|---|---|---|
| 🔲 | **MCP Server** | 将记忆作为 MCP tools 提供给 Claude / Cursor |
| 🔲 | **批量导入** | 加载现有 mem0 导出数据 |
| 🔲 | **多模态** | 图片/音频记忆支持 |
| 🔲 | **Agent SDK** | Rust crate 直接嵌入（无需 HTTP） |
| 🔲 | **Dashboard** | 轻量级 web UI 用于记忆检查 |

欢迎社区贡献 — 开 issue 或 PR 即可。

---

<div align="center">

**MIT License**

使用 [Claude Code](https://claude.ai) 构建

</div>
