<div align="center">

# R-Mem

### AI Agent 的長期記憶 — 以 Rust 實作

**以 Rust 研究 [mem0](https://github.com/mem0ai/mem0) 記憶架構的輕量實作。**<br>
**單一執行檔。SQLite 為後端。不需要 Python。**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)
[![Crates.io](https://img.shields.io/crates/v/rustmem.svg)](https://crates.io/crates/rustmem)
[![Awesome SQLite](https://img.shields.io/badge/Awesome-SQLite-green.svg)](https://github.com/planetopendata/awesome-sqlite)

[快速開始](#-快速開始) · [整合指南](#-整合指南) · [運作方式](#-運作方式) · [使用方式](#-使用方式) · [MCP](#-mcp-server) · [架構](#️-架構) · [路線圖](#️-路線圖)

🌐 [English](../README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

</div>

> [!NOTE]
> 本專案以學習為目的，用 Rust 重新實作 [mem0](https://github.com/mem0ai/mem0) 優雅的記憶架構。完全歸功於 mem0 團隊的原始設計。這不是替代品，而是以不同語言對其架構的研究。歡迎一起探討、交流與貢獻！

---

## 為什麼選擇 R-Mem？

mem0 是一個設計精良的記憶系統，擁有豐富的 plugin 生態系。R-Mem 問的是一個更窄的問題：*如果只把核心記憶邏輯用 Rust 重寫，並完全以 SQLite 為後端，會怎樣？*

結果是同樣的三層架構 — **vector memory**、**graph memory**、**history** — 加上 **tiered archive** 系統，以 **2,826 行 Rust** 實現。不需要外部服務。一個執行檔。取捨很明確：整合數量遠少於 mem0，但運維開銷趨近於零。

R-Mem 誕生自 [RustClaw](https://github.com/Adaimade/RustClaw) — 我們極簡風格的 Rust AI agent 框架。RustClaw 需要一個符合其理念的記憶層：單一執行檔、零外部服務。因此我們研究了 mem0 的架構，並以 Rust 重新實作。

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 執行檔</td><td>3.6 MB 靜態連結</td><td>Python + pip（豐富生態系）</td></tr>
<tr><td>💾 閒置 RSS</td><td>&lt; 10 MB</td><td>200 MB+（載入更多功能）</td></tr>
<tr><td>📝 程式碼</td><td>2,826 行</td><td>~91,500 行（26+ 種 store driver）</td></tr>
<tr><td>🔍 Vector</td><td>SQLite + FTS5</td><td>Qdrant、Chroma、Pinecone…</td></tr>
<tr><td>🕸️ Graph</td><td>僅 SQLite</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>OpenAI、Anthropic、Ollama</td><td>OpenAI、Anthropic 及更多</td></tr>
<tr><td>🗄️ Archive</td><td>分層記憶 + fallback 搜尋</td><td>—</td></tr>
</table>

> mem0 的數字反映的是它更豐富的生態系 — 更多 store、更多整合、更多彈性。R-Mem 有意犧牲這些來換取最小化的部署。

### R-Mem 在 mem0 之上新增的功能

| 功能 | R-Mem | mem0 |
|---|---|---|
| **分層 Archive** | 被刪除/更新的記憶保留 + fallback 搜尋 | 刪除即消失 |
| **FTS5 預過濾** | 兩階段搜尋：關鍵字 → vector（快 19 倍） | 僅 vector |
| **MCP Server** | 內建 `rustmem mcp`，支援 Claude Code / Cursor | 不提供 |
| **零依賴部署** | 單一執行檔、SQLite、無需 Docker | Python + pip + vector DB + graph DB |
| **Anthropic 原生支援** | 直接支援 Claude API | 透過 OpenAI 相容 proxy |
| **可配置管線** | `[memory]` 區段：門檻、上限等皆可調整 | 硬編碼預設值 |
| **記憶分類** | 自動分類：preference, personal, plan, professional, health | 無結構化 |

---

## 🔍 運作方式

```
Input text
│
├─ 📦 Vector Memory ──────────────────────────────────
│    │
│    ├─ LLM 萃取事實
│    │    → ["Name is Alice", "Works at Google"]
│    │
│    ├─ Embedding → cosine similarity 搜尋
│    │    （FTS5 預過濾 + vector 排序）
│    │
│    ├─ Integer ID mapping
│    │    （防止 LLM UUID 幻覺）
│    │
│    ├─ LLM 針對每個事實決策：
│    │    ├─ ADD       新資訊
│    │    ├─ UPDATE    更具體 → 舊版本歸檔
│    │    ├─ DELETE    矛盾 → 舊版本歸檔
│    │    └─ NONE      重複 — 跳過
│    │
│    └─ 執行動作 + 寫入歷史
│
├─ 🕸️ Graph Memory ──────────────────────────────────
│    │
│    ├─ LLM 萃取實體 + 關係
│    ├─ 衝突偵測（soft-delete 舊資料，新增新資料）
│    └─ 多值 vs 單值關係處理
│
└─ 🗄️ Archive ───────────────────────────────────────
     │
     ├─ 被刪除/取代的記憶保留 embedding
     ├─ 活躍搜尋結果不足時自動 fallback
     └─ 超過閾值自動壓縮
```

---

## 🚀 快速開始

### 前置條件

| 需求 | 安裝方式 |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM 後端 | [Ollama](https://ollama.com)、[OpenAI](https://platform.openai.com) 或 [Anthropic](https://console.anthropic.com) |

### 建置與執行

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem（3.6 MB）
```

### 設定

在專案根目錄建立 `rustmem.toml`：

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

### ⚠️ 建立 MemoryManager 還不夠

最常見的整合錯誤：你初始化了 `MemoryManager`，但在對話迴圈中從未呼叫 `add()` 或 `search()`。記憶系統存在但從未被使用 — 使用者說的任何話都不會被記住。

### 正確的對話迴圈

每一輪對話都必須包含兩個記憶操作：

1. **LLM 呼叫前 — 回憶**（搜尋相關記憶）
2. **LLM 呼叫後 — 學習**（萃取並儲存新事實）

```
loop {
    user_message = receive()

    // 1. 回憶 — 在呼叫 LLM 之前
    memories = rmem.search(user_id, user_message, limit=10)
    context = format_as_context(memories)

    // 2. 帶著記憶上下文呼叫 LLM
    response = llm.chat(system_prompt + context + user_message)

    // 3. 學習 — 在回應之後
    rmem.add(user_id, user_message)

    send(response)
}
```

### 記憶上下文格式

將 `search()` 結果格式化為 LLM 可以理解的上下文：

```
[Memory]
關於此使用者的已知事實：
- 使用者的名字是 Alice
- 使用者偏好深色模式
- 使用者正在開發一個 Rust 專案
```

將此放在 system prompt 中或使用者訊息之前，讓 LLM 可以參考。

### 多範圍模式

如果你的應用服務於多個頻道（例如 Telegram + Discord），使用三層範圍：

| 範圍 | 用途 | 範例 ID |
|---|---|---|
| local | 單一對話 / 群組 | `telegram:group_123` |
| user | 跨頻道的個人記憶 | `user:456` |
| global | 所有使用者共享 | `global:system` |

在回憶時合併結果：

```
local_results  = search("telegram:group_123", query)
user_results   = search("user:456", query)
global_results = search("global:system", query)
all = deduplicate(local + user + global)
```

### 常見錯誤

- ❌ 初始化 MemoryManager 但在迴圈中從未呼叫 `search()` / `add()`
- ❌ 只學習不回憶（記憶被儲存但從未被檢索）
- ❌ 只回憶不學習（讀取舊記憶但從不學習新的）
- ❌ 在 LLM 呼叫前執行 `add()`（當前訊息被當作已知事實）

---

## 📖 使用方式

### CLI

```bash
# 新增記憶
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."

# 語意搜尋
rustmem search -u alice "What does Alice eat?"

# 列出使用者所有記憶
rustmem list -u alice

# 顯示圖譜關係
rustmem graph -u alice

# 啟動 REST API 伺服器
rustmem server
```

### REST API

先啟動 `rustmem server`，然後：

```bash
# ➕ 新增記憶
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

# 🔍 搜尋
curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

# 📋 列出全部
curl http://localhost:8019/memories?user_id=alice

# 🏷️ 依分類過濾（preference, personal, plan, professional, health, misc）
curl http://localhost:8019/memories?user_id=alice&category=preference

# 🗑️ 刪除
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 歷史紀錄
curl http://localhost:8019/memories/{id}/history

# 🗄️ 查看歸檔記憶
curl http://localhost:8019/archive?user_id=alice

# 🕸️ 查看圖譜關係
curl http://localhost:8019/graph?user_id=alice
```

### AI Agent 直接替換

```python
# mem0（之前）
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem（之後 — 只需改用 HTTP）
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 🔌 MCP Server

R-Mem 可作為 MCP server — 一行指令即可讓 Claude Code 或 Cursor 擁有長期記憶：

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

**7 個 tools：** `add_memory`、`search_memory`、`list_memories`、`get_memory`、`delete_memory`、`get_graph`、`reset_memories`

---

## 🏗️ 架構

```
src/
├── main.rs          CLI 入口（clap）
├── config.rs        TOML + 環境變數設定
├── server.rs        REST API（axum）
├── mcp.rs           MCP server（rmcp）— 7 個 tools 走 stdio
├── memory.rs        核心協調器 — 分層記憶管線
├── extract.rs       LLM 呼叫：OpenAI + Anthropic native
├── embedding.rs     OpenAI 相容 embedding 客戶端
├── store.rs         SQLite vector store（WAL + FTS5 + archive）
└── graph.rs         SQLite graph store（soft-delete、多值關係）
```

**9 個檔案。2,826 行。3.6 MB binary。零外部服務。**

---

## 🗺️ 路線圖

| 狀態 | 功能 | 說明 |
|---|---|---|
| ✅ | **發布至 crates.io** | `cargo install rustmem` — 一行指令安裝 |
| ✅ | **MCP Server** | `rustmem mcp` — 7 個 tools 走 stdio，支援 Claude Code / Cursor |
| ✅ | **Tiered Archive** | 被刪除/更新的記憶保留 + fallback 搜尋 |
| ✅ | **FTS5 兩階段搜尋** | 關鍵字預過濾 + vector 重新排序 — 快 19 倍 |
| ✅ | **記憶分類** | 自動分類：preference, personal, plan, professional, health |
| ✅ | **Anthropic Native** | 直接支援 Claude API（不需 proxy） |
| ✅ | **Agent SDK（lib crate）** | 在 Rust 程式碼中直接使用 `rustmem::{memory, store, graph}` |
| ✅ | **LongMemEval Benchmark** | 使用 gpt-4o-mini 達 48.2%，接近 mem0（約 49%） |
| ✅ | **Production Audit** | 11 項安全性/穩定性修復、25 個單元測試、cargo bench |
| 🔲 | **Episodic Memory** | 任務執行歷史（tool 呼叫、參數、結果） |
| 🔲 | **User Preference Model** | 跨 session 的使用者風格與行為建模 |
| 🔲 | **Skill Abstraction** | 自動將重複成功模式萃取為 skill |
| 🔲 | **批次匯入** | 載入現有 mem0 匯出資料 |
| 🔲 | **多模態** | 圖片/音訊記憶支援 |
| 🔲 | **Dashboard** | 輕量級 web UI 用於記憶檢視 |

R-Mem v0.3.0 作為學習專案已功能完備。核心架構穩定且已通過生產級強化。歡迎社群貢獻、fork 和探索 — 開 issue 或 PR 即可。

---

<div align="center">

**MIT License** · v0.3.0

由 [Ad Huang](https://github.com/Adaimade) 使用 [Claude Code](https://claude.ai) 建置

</div>
