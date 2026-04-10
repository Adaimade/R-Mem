<div align="center">

# R-Mem

### AI Agent のための長期メモリ — Rust で実装

**[mem0](https://github.com/mem0ai/mem0) のメモリアーキテクチャを Rust で研究する軽量実装。**<br>
**シングルバイナリ。SQLite バックエンド。Python 不要。**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)
[![Crates.io](https://img.shields.io/crates/v/rustmem.svg)](https://crates.io/crates/rustmem)
[![Awesome SQLite](https://img.shields.io/badge/Awesome-SQLite-green.svg)](https://github.com/planetopendata/awesome-sqlite)

[クイックスタート](#-クイックスタート) · [仕組み](#-仕組み) · [使い方](#-使い方) · [MCP](#-mcp-server) · [アーキテクチャ](#️-アーキテクチャ) · [ロードマップ](#️-ロードマップ)

🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [한국어](README.ko.md)

</div>

> [!NOTE]
> 本プロジェクトは学習目的で、[mem0](https://github.com/mem0ai/mem0) の優れたメモリアーキテクチャを Rust で再実装したものです。オリジナルの設計は mem0 チームの功績です。これは代替品ではなく、異なる言語によるアーキテクチャの研究です。ディスカッション、アイデア、コントリビューションを歓迎します！

---

## なぜ R-Mem か？

mem0 は優れた設計のメモリシステムであり、豊富な plugin エコシステムを持っています。R-Mem はより狭い問いを立てています：*コアのメモリロジックだけを Rust で書き直し、完全に SQLite をバックエンドにしたらどうなるか？*

結果は同じ三層アーキテクチャ — **vector memory**、**graph memory**、**history** — と **階層アーカイブ** システムを **2,826 行の Rust** で実現。外部サービス不要。バイナリ一つ。トレードオフは明確：統合の数は mem0 よりはるかに少ないが、運用オーバーヘッドはほぼゼロ。

R-Mem は [RustClaw](https://github.com/Adaimade/RustClaw) から生まれました — 私たちのミニマリスト Rust AI agent フレームワークです。RustClaw にはその哲学に合ったメモリレイヤーが必要でした：シングルバイナリ、外部サービスゼロ。そこで mem0 のアーキテクチャを研究し、Rust で再構築しました。

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 バイナリ</td><td>3.6 MB 静的リンク</td><td>Python + pip（豊富なエコシステム）</td></tr>
<tr><td>💾 アイドル RSS</td><td>&lt; 10 MB</td><td>200 MB+（より多くの機能をロード）</td></tr>
<tr><td>📝 コード</td><td>2,826 行</td><td>~91,500 行（26+ 種の store driver）</td></tr>
<tr><td>🔍 Vector</td><td>SQLite + FTS5</td><td>Qdrant、Chroma、Pinecone…</td></tr>
<tr><td>🕸️ Graph</td><td>SQLite のみ</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>OpenAI、Anthropic、Ollama</td><td>OpenAI、Anthropic など</td></tr>
<tr><td>🗄️ Archive</td><td>階層メモリ + fallback 検索</td><td>—</td></tr>
</table>

> mem0 の数字は豊かなエコシステムを反映しています — より多くの store、より多くの統合、より多くの柔軟性。R-Mem は最小限のフットプリントのためにそれらを意図的にトレードオフしています。

### R-Mem が mem0 に追加する機能

| 機能 | R-Mem | mem0 |
|---|---|---|
| **階層アーカイブ** | 削除/更新されたメモリを保存 + fallback 検索 | 削除されたら消失 |
| **FTS5 プリフィルタ** | 2 段階検索：キーワード → vector（19 倍高速） | vector のみ |
| **MCP Server** | 内蔵 `rustmem mcp`、Claude Code / Cursor 対応 | 提供なし |
| **依存ゼロのデプロイ** | シングルバイナリ、SQLite、Docker 不要 | Python + pip + vector DB + graph DB |
| **Anthropic ネイティブ対応** | Claude API を直接サポート | OpenAI 互換プロキシ経由 |
| **設定可能なパイプライン** | `[memory]` セクション：閾値や上限などすべて調整可能 | ハードコードされたデフォルト |
| **メモリカテゴリ** | 自動分類：preference, personal, plan, professional, health | 非構造化 |

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
│    ├─ Embedding → cosine similarity 検索
│    │    （FTS5 プリフィルタ + vector ランキング）
│    │
│    ├─ Integer ID mapping
│    │    （LLM の UUID ハルシネーション防止）
│    │
│    ├─ LLM が各事実について判定：
│    │    ├─ ADD       新しい情報
│    │    ├─ UPDATE    より具体的 → 旧バージョンをアーカイブ
│    │    ├─ DELETE    矛盾 → 旧バージョンをアーカイブ
│    │    └─ NONE      重複 — スキップ
│    │
│    └─ アクション実行 + 履歴に書き込み
│
├─ 🕸️ Graph Memory ──────────────────────────────────
│    │
│    ├─ LLM がエンティティ + 関係を抽出
│    ├─ 競合検出（古いデータを soft-delete、新しいデータを追加）
│    └─ 多値 vs 単値の関係処理
│
└─ 🗄️ Archive ───────────────────────────────────────
     │
     ├─ 削除/更新されたメモリを embedding 付きで保存
     ├─ アクティブな結果が弱い場合の fallback 検索
     └─ アーカイブが閾値を超えると自動圧縮
```

---

## 🚀 クイックスタート

### 前提条件

| 要件 | インストール |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM バックエンド | [Ollama](https://ollama.com)（ローカル）、[OpenAI](https://platform.openai.com)、または [Anthropic](https://console.anthropic.com) |

### ビルド & 実行

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem（3.6 MB）
```

### 設定

プロジェクトルートに `rustmem.toml` を作成：

<table>
<tr>
<td><strong>Ollama（ローカル）</strong></td>
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

> **注意：** Anthropic は embedding モデルを提供していないため、`[llm]` で Anthropic を使用する場合でも `[embedding]` は OpenAI または Ollama を使用します。

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

# 🏷️ カテゴリでフィルタ（preference, personal, plan, professional, health, misc）
curl http://localhost:8019/memories?user_id=alice&category=preference

# 🗑️ 削除
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 履歴
curl http://localhost:8019/memories/{id}/history

# 🗄️ アーカイブされたメモリを表示
curl http://localhost:8019/archive?user_id=alice

# 🕸️ グラフ関係を表示
curl http://localhost:8019/graph?user_id=alice
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
├── memory.rs        コアオーケストレータ — 階層メモリパイプライン
├── extract.rs       LLM 呼び出し：OpenAI + Anthropic native
├── embedding.rs     OpenAI 互換 embedding クライアント
├── store.rs         SQLite vector store（WAL + FTS5 + archive）
└── graph.rs         SQLite graph store（soft-delete、多値関係）
```

**9 ファイル。2,826 行。3.6 MB バイナリ。外部サービスゼロ。**

---

## 🗺️ ロードマップ

| ステータス | 機能 | 説明 |
|---|---|---|
| ✅ | **crates.io に公開** | `cargo install rustmem` — ワンライナーインストール |
| ✅ | **MCP Server** | `rustmem mcp` — stdio 経由の 7 tools、Claude Code / Cursor 対応 |
| ✅ | **階層アーカイブ** | 削除/更新されたメモリを保存 + fallback 検索 |
| ✅ | **FTS5 2 段階検索** | キーワードプリフィルタ + vector 再ランキング — 19 倍高速 |
| ✅ | **メモリカテゴリ** | 自動分類：preference, personal, plan, professional, health |
| ✅ | **Anthropic Native** | Claude API の直接サポート（プロキシ不要） |
| ✅ | **Agent SDK（lib crate）** | Rust コードから `rustmem::{memory, store, graph}` を直接使用 |
| ✅ | **LongMemEval Benchmark** | gpt-4o-mini で 48.2%、mem0（約 49%）にほぼ匹敵 |
| ✅ | **Production Audit** | 11 項目のセキュリティ/安定性修正、25 個のユニットテスト、cargo bench |
| 🔲 | **Episodic Memory** | タスク実行履歴（tool 呼び出し、パラメータ、結果） |
| 🔲 | **User Preference Model** | セッション横断のユーザースタイルと行動モデリング |
| 🔲 | **Skill Abstraction** | 繰り返される成功パターンを skill として自動抽出 |
| 🔲 | **バッチインポート** | 既存の mem0 エクスポートデータを読み込み |
| 🔲 | **マルチモーダル** | 画像/音声メモリサポート |
| 🔲 | **Dashboard** | メモリ検査用の軽量 web UI |

コミュニティからの貢献を歓迎します — issue または PR をお気軽にどうぞ。

---

<div align="center">

**MIT License** · v0.3.0

[Ad Huang](https://github.com/Adaimade) が [Claude Code](https://claude.ai) で構築

</div>
