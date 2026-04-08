<div align="center">

# R-Mem

### AI Agent のための長期メモリ — Rust で実装

**[mem0](https://github.com/mem0ai/mem0) のメモリアーキテクチャを Rust で研究する軽量実装。**<br>
**シングルバイナリ。SQLite バックエンド。Python 不要。**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)

[クイックスタート](#-クイックスタート) · [仕組み](#-仕組み) · [使い方](#-使い方) · [MCP](#-mcp-server) · [アーキテクチャ](#️-アーキテクチャ) · [ロードマップ](#️-ロードマップ)

🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [한국어](README.ko.md)

</div>

> [!NOTE]
> 本プロジェクトは学習目的で、[mem0](https://github.com/mem0ai/mem0) の優れたメモリアーキテクチャを Rust で再実装したものです。オリジナルの設計は mem0 チームの功績です。これは代替品ではなく、異なる言語によるアーキテクチャの研究です。ディスカッション、アイデア、コントリビューションを歓迎します！

---

## なぜ R-Mem か？

mem0 は優れた設計のメモリシステムであり、豊富な plugin エコシステムを持っています。R-Mem はより狭い問いを立てています：*コアのメモリロジックだけを Rust で書き直し、完全に SQLite をバックエンドにしたらどうなるか？*

結果は同じ三層アーキテクチャ — **vector memory**、**graph memory**、**history** — を **1,748 行の Rust** で実現。外部サービス不要。バイナリ一つ。トレードオフは明確：統合の数は mem0 よりはるかに少ないが、運用オーバーヘッドはほぼゼロ。

R-Mem は [RustClaw](https://github.com/Adaimade/RustClaw) から生まれました — 私たちのミニマリスト Rust AI agent フレームワークです。RustClaw にはその哲学に合ったメモリレイヤーが必要でした：シングルバイナリ、外部サービスゼロ。そこで mem0 のアーキテクチャを研究し、Rust で再構築しました。

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 バイナリ</td><td>3.2 MB 静的リンク</td><td>Python + pip（豊富なエコシステム）</td></tr>
<tr><td>💾 アイドル RSS</td><td>&lt; 10 MB</td><td>200 MB+（より多くの機能をロード）</td></tr>
<tr><td>📝 コード</td><td>1,748 行</td><td>~91,500 行（26+ 種の store driver）</td></tr>
<tr><td>🔍 Vector</td><td>SQLite のみ</td><td>Qdrant、Chroma、Pinecone…</td></tr>
<tr><td>🕸️ Graph</td><td>SQLite のみ</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>任意の OpenAI 互換エンドポイント（Ollama）</td><td>OpenAI、Anthropic など</td></tr>
</table>

> mem0 の数字は豊かなエコシステムを反映しています — より多くの store、より多くの統合、より多くの柔軟性。R-Mem は最小限のフットプリントのためにそれらを意図的にトレードオフしています。

---

## 🔍 仕組み

```
Input text
│
├─ 📦 Vector Memory ──────────────────────────────────
│    │
│    ├─ LLM が事実を抽出
│    │    → ["Name is Alice", "Works at Google"]
│    │
│    ├─ Embedding → cosine similarity 検索（上位 5 件）
│    │
│    ├─ Integer ID mapping
│    │    （LLM の UUID ハルシネーション防止）
│    │
│    ├─ LLM が各事実について判定：
│    │    ├─ ADD       新しい情報
│    │    ├─ UPDATE    より具体的
│    │    │             "likes sports" → "likes tennis"
│    │    ├─ DELETE    矛盾
│    │    │             "likes pizza" → "hates pizza"
│    │    └─ NONE      重複 — スキップ
│    │
│    └─ アクション実行 + 履歴に書き込み
│
└─ 🕸️ Graph Memory ──────────────────────────────────
     │
     ├─ LLM がエンティティ + 関係を抽出
     ├─ 競合検出（古いデータを soft-delete、新しいデータを追加）
     └─ 多値 vs 単値の関係処理
```

---

## 🚀 クイックスタート

### 前提条件

| 要件 | インストール |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM バックエンド | [Ollama](https://ollama.com)（ローカル）または任意の OpenAI 互換 API |

### ビルド & 実行

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem（3.2 MB）
```

### 設定

プロジェクトルートに `rustmem.toml` を作成：

<table>
<tr>
<td><strong>Ollama（ローカル）</strong></td>
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

## 📖 使い方

### CLI

```bash
# メモリを追加
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."

# セマンティック検索
rustmem search -u alice "What does Alice eat?"

# ユーザーの全メモリを一覧
rustmem list -u alice

# グラフ関係を表示
rustmem graph -u alice

# REST API サーバーを起動
rustmem server
```

### REST API

`rustmem server` で起動後：

```bash
# ➕ メモリを追加
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

# 🔍 検索
curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

# 📋 全件取得
curl http://localhost:8019/memories?user_id=alice

# 🗑️ 削除
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 履歴
curl http://localhost:8019/memories/{id}/history
```

### AI Agent へのドロップイン

```python
# mem0（以前）
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem（以後 — HTTP に切り替えるだけ）
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 🔌 MCP Server

R-Mem は MCP server として動作します — 1 コマンドで Claude Code や Cursor に長期メモリを付与：

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

**7 つの tools：** `add_memory`、`search_memory`、`list_memories`、`get_memory`、`delete_memory`、`get_graph`、`reset_memories`

---

## 🏗️ アーキテクチャ

```
src/
├── main.rs          CLI エントリポイント（clap）
├── config.rs        TOML + 環境変数設定
├── server.rs        REST API（axum）
├── mcp.rs           MCP server（rmcp）— stdio 経由の 7 tools
├── memory.rs        コアオーケストレータ — 3層メモリパイプライン
├── extract.rs       LLM prompts：事実/エンティティ/関係抽出
├── embedding.rs     OpenAI 互換 embedding クライアント
├── store.rs         SQLite vector store（cosine similarity）
└── graph.rs         SQLite graph store（soft-delete、多値関係）
```

**9 ファイル。1,748 行。外部サービスゼロ。**

---

## 🗺️ ロードマップ

| ステータス | 機能 | 説明 |
|---|---|---|
| ✅ | **MCP Server** | `rustmem mcp` — stdio 経由の 7 tools、Claude Code / Cursor 対応 |
| 🔲 | **バッチインポート** | 既存の mem0 エクスポートデータを読み込み |
| 🔲 | **マルチモーダル** | 画像/音声メモリサポート |
| 🔲 | **Agent SDK** | Rust crate による直接埋め込み（HTTP 不要） |
| 🔲 | **Dashboard** | メモリ検査用の軽量 web UI |

コミュニティからの貢献を歓迎します — issue または PR をお気軽にどうぞ。

---

<div align="center">

**MIT License**

[Ad Huang](https://github.com/Adaimade) が [Claude Code](https://claude.ai) で構築

</div>
