🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [한국어](README.ko.md)

# R-Mem

**mem0 の Rust 実装。AI agent のための長期メモリ。シングルバイナリ。Python 不要。**

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| バイナリ / Runtime | **3.2 MB** 静的リンク | Python + pip が必要          |
| アイドルメモリ (RSS) | **< 10 MB**       | 200 MB+                      |
| コード行数         | **1,748**          | ~91,500                      |
| Vector Store      | SQLite（組み込み）   | Qdrant + 26 種以上            |
| Graph Store       | SQLite（組み込み）   | Neo4j / Memgraph             |
| 依存関係           | コンパイル時に内包    | pip install mem0ai           |
| LLM バックエンド   | 任意の OpenAI 互換エンドポイント（Ollama） | OpenAI / Anthropic のみ |

---

## なぜ R-Mem か

mem0 は強力です。しかし 91,500 行の Python コードがあり、稼働中のベクトルデータベースが必要で、何もする前に 200MB+ の RAM を消費します。

R-Mem は同じ三層メモリアーキテクチャ — vector memory、graph memory、history — を 1,748 行の Rust で実現しています。SQLite がベクトルとグラフの両方のストレージを処理します。外部サービス不要。ランタイム不要。バイナリ一つで完結。

Claude Code で完全に構築。

> **注意：** 本プロジェクトは、AI メモリシステムを Rust で再実装する研究です。コアロジックとアーキテクチャは [mem0](https://github.com/mem0ai/mem0) を参考にしています。ディスカッション、アイデア、コントリビューションを歓迎します！

---

## 仕組み

```
Input text
│
├── Vector Memory（長期的な事実）
│     ├── LLM が事実を抽出 → ["Name is Alice", "Works at Google"]
│     ├── Embedding → cosine similarity 検索（既存の上位 5 件）
│     ├── Integer ID mapping（LLM の UUID ハルシネーション防止）
│     ├── LLM が各事実について判定：
│     │     ├── ADD       → 新しい情報
│     │     ├── UPDATE    → より具体的（"likes sports" → "likes tennis"）
│     │     ├── DELETE    → 矛盾（"likes pizza" vs "hates pizza"）
│     │     └── NONE      → 重複、スキップ
│     └── 実行 + 履歴に書き込み
│
└── Graph Memory（エンティティ関係）
      ├── LLM がエンティティ + 関係を抽出
      ├── 競合検出（古いデータを soft-delete、新しいデータを追加）
      └── 多値 vs 単値の関係処理
```

---

## クイックスタート

### 前提条件

- Rust ツールチェーン（`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`）
- LLM バックエンド：[Ollama](https://ollama.com)（ローカル）または任意の OpenAI 互換エンドポイント

### ビルド

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# バイナリ：target/release/rustmem（3.2 MB）
```

### 設定

`rustmem.toml` を作成：

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

または OpenAI を使用：

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

## 使い方

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

## アーキテクチャ

```
src/
├── main.rs        # CLI (clap)
├── config.rs      # TOML + 環境変数設定
├── server.rs      # REST API (axum)
├── memory.rs      # コアオーケストレータ
├── extract.rs     # LLM prompts：事実/エンティティ/関係抽出
├── embedding.rs   # OpenAI 互換 embedding クライアント
├── store.rs       # SQLite vector store（cosine similarity）
└── graph.rs       # SQLite graph store（soft-delete、多値関係）
```

---

## AI Agent との統合

```python
# mem0（以前）
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem（以後 — HTTP 経由でドロップイン置換）
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## ロードマップ

- [ ] MCP server — メモリを MCP tools として Claude / Cursor に提供
- [ ] バッチインポート — 既存の mem0 エクスポートデータを読み込み
- [ ] マルチモーダル — 画像/音声メモリサポート
- [ ] Agent SDK — Rust crate による直接埋め込み（HTTP 不要）
- [ ] Dashboard — メモリ検査用の軽量 web UI

コミュニティからの貢献を歓迎します。issue または PR をお気軽にどうぞ。

---

## ライセンス

MIT
