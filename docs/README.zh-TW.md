🌐 [English](../README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md) · [한국어](README.ko.md)

# R-Mem

**mem0 的 Rust 實作。AI agent 的長期記憶。單一執行檔。不需要 Python。**

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| 執行檔 / Runtime  | **3.2 MB** 靜態連結 | 需要 Python + pip            |
| 閒置記憶體 (RSS)   | **< 10 MB**        | 200 MB+                      |
| 程式碼行數         | **1,748**          | ~91,500                      |
| Vector Store      | SQLite（內建）      | Qdrant + 26 種以上            |
| Graph Store       | SQLite（內建）      | Neo4j / Memgraph             |
| 依賴               | 編譯時內含          | pip install mem0ai           |
| LLM 後端          | 任何 OpenAI 相容端點（Ollama） | 僅 OpenAI / Anthropic |

---

## 為什麼

mem0 很強大。但它有 91,500 行 Python 程式碼、需要執行中的向量資料庫，而且在做任何事之前就會消耗 200MB+ 的記憶體。

R-Mem 是同樣的三層記憶架構 — vector memory、graph memory、history — 只用 1,748 行 Rust 實現。SQLite 同時處理向量和圖儲存。不需要外部服務。不需要 runtime。一個執行檔搞定。

完全使用 Claude Code 建置。

> **注意：** 本專案是一項基於 Rust 寫法重新實作 AI 記憶系統的研究。核心邏輯與架構參考自 [mem0](https://github.com/mem0ai/mem0)。歡迎一起探討、交流與貢獻！

---

## 運作方式

```
Input text
│
├── Vector Memory（長期事實）
│     ├── LLM 萃取事實 → ["Name is Alice", "Works at Google"]
│     ├── Embedding → cosine similarity 搜尋（前 5 筆現有記憶）
│     ├── Integer ID mapping（防止 LLM UUID 幻覺）
│     ├── LLM 針對每個事實決策：
│     │     ├── ADD       → 新資訊
│     │     ├── UPDATE    → 更具體（"likes sports" → "likes tennis"）
│     │     ├── DELETE    → 矛盾（"likes pizza" vs "hates pizza"）
│     │     └── NONE      → 重複，跳過
│     └── 執行 + 寫入歷史
│
└── Graph Memory（實體關係）
      ├── LLM 萃取實體 + 關係
      ├── 衝突偵測（soft-delete 舊資料，新增新資料）
      └── 多值 vs 單值關係處理
```

---

## 快速開始

### 前置條件

- Rust 工具鏈（`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`）
- LLM 後端：[Ollama](https://ollama.com)（本地）或任何 OpenAI 相容端點

### 建置

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# 執行檔：target/release/rustmem（3.2 MB）
```

### 設定

建立 `rustmem.toml`：

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

## 架構

```
src/
├── main.rs        # CLI (clap)
├── config.rs      # TOML + 環境變數設定
├── server.rs      # REST API (axum)
├── memory.rs      # 核心協調器
├── extract.rs     # LLM prompts：事實/實體/關係萃取
├── embedding.rs   # OpenAI 相容 embedding 客戶端
├── store.rs       # SQLite vector store（cosine similarity）
└── graph.rs       # SQLite graph store（soft-delete、多值關係）
```

---

## 與 AI Agent 整合

```python
# mem0（之前）
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem（之後 — 透過 HTTP 直接替換）
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 路線圖

- [ ] MCP server — 將記憶作為 MCP tools 提供給 Claude / Cursor
- [ ] 批次匯入 — 載入現有 mem0 匯出資料
- [ ] 多模態 — 圖片/音訊記憶支援
- [ ] Agent SDK — Rust crate 直接嵌入（不需 HTTP）
- [ ] Dashboard — 輕量級 web UI 用於記憶檢視

歡迎社群貢獻。開 issue 或 PR 即可。

---

## 授權條款

MIT
