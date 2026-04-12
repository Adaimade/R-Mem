<div align="center">

# R-Mem

### AI Agent 的长期记忆 — 以 Rust 实现

**以 Rust 研究 [mem0](https://github.com/mem0ai/mem0) 记忆架构的轻量实现。**<br>
**单一可执行文件。SQLite 为后端。无需 Python。**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Crates.io](https://img.shields.io/crates/v/rustmem.svg)](https://crates.io/crates/rustmem)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)
[![Awesome SQLite](https://img.shields.io/badge/Awesome-SQLite-green.svg)](https://github.com/planetopendata/awesome-sqlite)

[快速开始](#-快速开始) · [整合指南](#-整合指南) · [工作原理](#-工作原理) · [使用方式](#-使用方式) · [MCP](#-mcp-server) · [架构](#️-架构) · [路线图](#️-路线图)

🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

> [!NOTE]
> 本项目以学习为目的，用 Rust 重新实现 [mem0](https://github.com/mem0ai/mem0) 优雅的记忆架构。完全归功于 mem0 团队的原始设计。这不是替代品，而是以不同语言对其架构的研究。欢迎一起探讨、交流与贡献！

---

## 为什么选择 R-Mem？

mem0 是一个设计精良的记忆系统，拥有丰富的 plugin 生态系统。R-Mem 问的是一个更窄的问题：*如果只把核心记忆逻辑用 Rust 重写，并完全以 SQLite 为后端，会怎样？*

结果是同样的三层架构 — **vector memory**、**graph memory**、**history** — 加上**分层 archive** 系统，以 **2,826 行 Rust** 实现。无需外部服务。一个可执行文件。取舍很明确：集成数量远少于 mem0，但运维开销趋近于零。

R-Mem 诞生自 [RustClaw](https://github.com/Adaimade/RustClaw) — 我们极简风格的 Rust AI agent 框架。RustClaw 需要一个符合其理念的记忆层：单一可执行文件、零外部服务。因此我们研究了 mem0 的架构，并以 Rust 重新实现。

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 可执行文件</td><td>3.6 MB 静态链接</td><td>Python + pip（丰富生态系统）</td></tr>
<tr><td>💾 空闲 RSS</td><td>&lt; 10 MB</td><td>200 MB+（加载更多功能）</td></tr>
<tr><td>📝 代码</td><td>2,826 行</td><td>~91,500 行（26+ 种 store driver）</td></tr>
<tr><td>🔍 Vector</td><td>SQLite + FTS5</td><td>Qdrant、Chroma、Pinecone…</td></tr>
<tr><td>🕸️ Graph</td><td>仅 SQLite</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>OpenAI、Anthropic、Ollama</td><td>OpenAI、Anthropic 及更多</td></tr>
<tr><td>🗄️ Archive</td><td>分层记忆 + fallback 搜索</td><td>—</td></tr>
</table>

> mem0 的数字反映的是它更丰富的生态系统 — 更多 store、更多集成、更多灵活性。R-Mem 有意牺牲这些来换取最小化的部署。

### R-Mem 在 mem0 之上新增的功能

| 功能 | R-Mem | mem0 |
|---|---|---|
| **分层 Archive** | 已删除/更新的记忆保留 + fallback 搜索 | 删除即消失 |
| **FTS5 预过滤** | 两阶段搜索：关键词 → vector（快 19 倍） | 仅 vector |
| **MCP Server** | 内置 `rustmem mcp`，支持 Claude Code / Cursor | 不提供 |
| **零依赖部署** | 单一可执行文件、SQLite、无需 Docker | Python + pip + vector DB + graph DB |
| **Anthropic 原生支持** | 直接支持 Claude API | 通过 OpenAI 兼容 proxy |
| **可配置管线** | `[memory]` 区块：阈值、上限等均可调整 | 硬编码默认值 |
| **记忆分类** | 自动分类：preference, personal, plan, professional, health | 无结构化 |

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
│    ├─ Embedding → cosine similarity 搜索
│    │    （FTS5 预过滤 + vector 排序）
│    │
│    ├─ Integer ID mapping
│    │    （防止 LLM UUID 幻觉）
│    │
│    ├─ LLM 针对每个事实决策：
│    │    ├─ ADD       新信息
│    │    ├─ UPDATE    更具体 → 旧版本归档
│    │    │             "likes sports" → "likes tennis"
│    │    ├─ DELETE    矛盾 → 旧版本归档
│    │    │             "likes pizza" → "hates pizza"
│    │    └─ NONE      重复 — 跳过
│    │
│    └─ 执行动作 + 写入历史
│
├─ 🕸️ Graph Memory ──────────────────────────────────
│    │
│    ├─ LLM 提取实体 + 关系
│    ├─ 冲突检测（soft-delete 旧数据，添加新数据）
│    └─ 多值 vs 单值关系处理
│
└─ 🗄️ Archive ───────────────────────────────────────
     │
     ├─ 已删除/被替代的记忆连同 embedding 一起保留
     ├─ 当活跃结果不足时进行 fallback 搜索
     └─ Archive 超过阈值时自动压缩
```

---

## 🚀 快速开始

### 前置条件

| 需求 | 安装方式 |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM 后端 | [Ollama](https://ollama.com)（本地）、[OpenAI](https://platform.openai.com) 或 [Anthropic](https://console.anthropic.com) |

### 构建与运行

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem（3.6 MB）
```

### 配置

在项目根目录创建 `rustmem.toml`：

<table>
<tr>
<td><strong>Ollama（本地）</strong></td>
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

> **注意：** Anthropic 不提供 embedding 模型，因此 [embedding] 即使 [llm] 使用 Anthropic 也需要用 OpenAI 或 Ollama。

---

## 🔗 整合指南

### ⚠️ 构建 MemoryManager 还不够

最常见的整合错误：你初始化了 `MemoryManager`，但在对话循环中从未调用 `add()` 或 `search()`。记忆系统存在但从未被使用 — 用户说的任何话都不会被记住。

### 正确的对话循环

每一轮对话都必须包含两个记忆操作：

1. **LLM 调用前 — 回忆**（搜索相关记忆）
2. **LLM 调用后 — 学习**（提取并存储新事实）

```
loop {
    user_message = receive()

    // 1. 回忆 — 在调用 LLM 之前
    memories = rmem.search(user_id, user_message, limit=10)
    context = format_as_context(memories)

    // 2. 带着记忆上下文调用 LLM
    response = llm.chat(system_prompt + context + user_message)

    // 3. 学习 — 在响应之后
    rmem.add(user_id, user_message)

    send(response)
}
```

### 记忆上下文格式

将 `search()` 结果格式化为 LLM 可以理解的上下文：

```
[Memory]
关于此用户的已知事实：
- 用户的名字是 Alice
- 用户偏好深色模式
- 用户正在开发一个 Rust 项目
```

将此放在 system prompt 中或用户消息之前，让 LLM 可以引用。

### 多范围模式

如果你的应用服务于多个频道（例如 Telegram + Discord），使用三层范围：

| 范围 | 用途 | 示例 ID |
|---|---|---|
| local | 单个对话 / 群组 | `telegram:group_123` |
| user | 跨频道的个人记忆 | `user:456` |
| global | 所有用户共享 | `global:system` |

在回忆时合并结果：

```
local_results  = search("telegram:group_123", query)
user_results   = search("user:456", query)
global_results = search("global:system", query)
all = deduplicate(local + user + global)
```

### 常见错误

- ❌ 初始化 MemoryManager 但在循环中从未调用 `search()` / `add()`
- ❌ 只学习不回忆（记忆被存储但从未被检索）
- ❌ 只回忆不学习（读取旧记忆但从不学习新的）
- ❌ 在 LLM 调用前执行 `add()`（当前消息被当作已知事实）

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

# 🏷️ 按分类过滤（preference, personal, plan, professional, health, misc）
curl http://localhost:8019/memories?user_id=alice&category=preference

# 🗑️ 删除
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 历史记录
curl http://localhost:8019/memories/{id}/history

# 🗄️ 查看归档记忆
curl http://localhost:8019/archive?user_id=alice

# 🕸️ 查看图谱关系
curl http://localhost:8019/graph?user_id=alice
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

## 🔌 MCP Server

R-Mem 可作为 MCP server — 一行命令即可让 Claude Code 或 Cursor 拥有长期记忆：

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

**7 个 tools：** `add_memory`、`search_memory`、`list_memories`、`get_memory`、`delete_memory`、`get_graph`、`reset_memories`

---

## 🏗️ 架构

```
src/
├── main.rs          CLI 入口（clap）
├── config.rs        TOML + 环境变量配置
├── server.rs        REST API（axum）
├── mcp.rs           MCP server（rmcp）— 7 个 tools 走 stdio
├── memory.rs        核心协调器 — 分层记忆管线
├── extract.rs       LLM 调用：OpenAI + Anthropic native
├── embedding.rs     OpenAI 兼容 embedding 客户端
├── store.rs         SQLite vector store（WAL + FTS5 + archive）
└── graph.rs         SQLite graph store（soft-delete、多值关系）
```

**9 个文件。2,826 行。3.6 MB binary。零外部服务。**

---

## 🗺️ 路线图

| 状态 | 功能 | 说明 |
|---|---|---|
| ✅ | **发布至 crates.io** | `cargo install rustmem` — 一行命令安装 |
| ✅ | **MCP Server** | `rustmem mcp` — 7 个 tools 走 stdio，支持 Claude Code / Cursor |
| ✅ | **分层 Archive** | 已删除/更新的记忆保留 + fallback 搜索 |
| ✅ | **FTS5 两阶段搜索** | 关键词预过滤 + vector 重新排序 — 快 19 倍 |
| ✅ | **记忆分类** | 自动分类：preference, personal, plan, professional, health |
| ✅ | **Anthropic Native** | 直接支持 Claude API（无需代理） |
| ✅ | **Agent SDK（lib crate）** | 在 Rust 代码中直接使用 `rustmem::{memory, store, graph}` |
| ✅ | **LongMemEval Benchmark** | 使用 gpt-4o-mini 达 48.2%，接近 mem0（约 49%） |
| ✅ | **Production Audit** | 11 项安全/稳定性修复、25 个单元测试、cargo bench |
| 🔲 | **Episodic Memory** | 任务执行历史（tool 调用、参数、结果） |
| 🔲 | **User Preference Model** | 跨 session 的用户风格与行为建模 |
| 🔲 | **Skill Abstraction** | 自动将重复成功模式提取为 skill |
| 🔲 | **批量导入** | 加载现有 mem0 导出数据 |
| 🔲 | **多模态** | 图片/音频记忆支持 |
| 🔲 | **Dashboard** | 轻量级 web UI 用于记忆检查 |

欢迎社区贡献 — 开 issue 或 PR 即可。

---

<div align="center">

**MIT License** · v0.3.0

由 [Ad Huang](https://github.com/Adaimade) 使用 [Claude Code](https://claude.ai) 构建

</div>
