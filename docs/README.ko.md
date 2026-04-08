<div align="center">

# R-Mem

### AI Agent를 위한 장기 메모리 — Rust로 구현

**[mem0](https://github.com/mem0ai/mem0)의 메모리 아키텍처를 Rust로 연구하는 경량 구현.**<br>
**단일 바이너리. SQLite 백엔드. Python 불필요.**

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![Built with Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blueviolet)](https://claude.ai)

[빠른 시작](#-빠른-시작) · [작동 방식](#-작동-방식) · [사용법](#-사용법) · [아키텍처](#️-아키텍처) · [로드맵](#️-로드맵)

🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

</div>

> [!NOTE]
> 이 프로젝트는 학습 목적으로 [mem0](https://github.com/mem0ai/mem0)의 우아한 메모리 아키텍처를 Rust로 재구현한 것입니다. 원본 설계에 대한 공은 전적으로 mem0 팀에게 있습니다. 이것은 대체품이 아니라 다른 언어로 접근하는 아키텍처 연구입니다. 토론, 아이디어, 기여를 환영합니다!

---

## 왜 R-Mem인가?

mem0는 잘 설계된 메모리 시스템으로 풍부한 plugin 에코시스템을 가지고 있습니다. R-Mem은 더 좁은 질문을 던집니다: *핵심 메모리 로직만 Rust로 다시 작성하고, 완전히 SQLite를 백엔드로 사용하면 어떻게 될까?*

결과는 동일한 3계층 아키텍처 — **vector memory**, **graph memory**, **history** — 를 **1,748줄의 Rust**로 구현. 외부 서비스 불필요. 바이너리 하나. 트레이드오프는 명확: 통합 수는 mem0보다 훨씬 적지만, 운영 오버헤드는 거의 제로.

R-Mem은 [RustClaw](https://github.com/Adaimade/RustClaw)에서 탄생했습니다 — 우리의 미니멀리스트 Rust AI agent 프레임워크입니다. RustClaw에는 그 철학에 맞는 메모리 레이어가 필요했습니다: 단일 바이너리, 외부 서비스 제로. 그래서 mem0의 아키텍처를 연구하고 Rust로 재구축했습니다.

<table>
<tr><td></td><td><strong>R-Mem</strong></td><td><strong>mem0</strong></td></tr>
<tr><td>📦 바이너리</td><td>3.2 MB 정적 링크</td><td>Python + pip (풍부한 에코시스템)</td></tr>
<tr><td>💾 유휴 RSS</td><td>&lt; 10 MB</td><td>200 MB+ (더 많은 기능 로드)</td></tr>
<tr><td>📝 코드</td><td>1,748 줄</td><td>~91,500 줄 (26+ 종 store driver)</td></tr>
<tr><td>🔍 Vector</td><td>SQLite만</td><td>Qdrant, Chroma, Pinecone…</td></tr>
<tr><td>🕸️ Graph</td><td>SQLite만</td><td>Neo4j / Memgraph</td></tr>
<tr><td>🤖 LLM</td><td>모든 OpenAI 호환 엔드포인트 (Ollama)</td><td>OpenAI, Anthropic 등</td></tr>
</table>

> mem0의 수치는 풍부한 에코시스템을 반영합니다 — 더 많은 store, 더 많은 통합, 더 많은 유연성. R-Mem은 최소한의 풋프린트를 위해 이를 의도적으로 트레이드오프합니다.

---

## 🔍 작동 방식

```
Input text
│
├─ 📦 Vector Memory ──────────────────────────────────
│    │
│    ├─ LLM이 사실 추출
│    │    → ["Name is Alice", "Works at Google"]
│    │
│    ├─ Embedding → cosine similarity 검색 (상위 5개)
│    │
│    ├─ Integer ID mapping
│    │    (LLM UUID 할루시네이션 방지)
│    │
│    ├─ LLM이 각 사실에 대해 판정:
│    │    ├─ ADD       새로운 정보
│    │    ├─ UPDATE    더 구체적
│    │    │             "likes sports" → "likes tennis"
│    │    ├─ DELETE    모순
│    │    │             "likes pizza" → "hates pizza"
│    │    └─ NONE      중복 — 건너뛰기
│    │
│    └─ 액션 실행 + 히스토리 기록
│
└─ 🕸️ Graph Memory ──────────────────────────────────
     │
     ├─ LLM이 엔티티 + 관계 추출
     ├─ 충돌 감지 (기존 데이터 soft-delete, 새 데이터 추가)
     └─ 다중값 vs 단일값 관계 처리
```

---

## 🚀 빠른 시작

### 사전 요구사항

| 요구사항 | 설치 |
|---|---|
| Rust 1.75+ | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLM 백엔드 | [Ollama](https://ollama.com) (로컬) 또는 모든 OpenAI 호환 API |

### 빌드 & 실행

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# → target/release/rustmem (3.2 MB)
```

### 설정

프로젝트 루트에 `rustmem.toml` 생성:

<table>
<tr>
<td><strong>Ollama (로컬)</strong></td>
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

## 📖 사용법

### CLI

```bash
# 메모리 추가
rustmem add -u alice "My name is Alice and I work at Google. I love sushi."

# 시맨틱 검색
rustmem search -u alice "What does Alice eat?"

# 사용자의 모든 메모리 목록
rustmem list -u alice

# 그래프 관계 표시
rustmem graph -u alice

# REST API 서버 시작
rustmem server
```

### REST API

`rustmem server`로 시작 후:

```bash
# ➕ 메모리 추가
curl -X POST http://localhost:8019/memories/add \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "text": "I moved to Tokyo last month"}'

# 🔍 검색
curl -X POST http://localhost:8019/memories/search \
  -H 'Content-Type: application/json' \
  -d '{"user_id": "alice", "query": "where does she live", "limit": 5}'

# 📋 전체 목록
curl http://localhost:8019/memories?user_id=alice

# 🗑️ 삭제
curl -X DELETE http://localhost:8019/memories/{id}

# 📜 히스토리
curl http://localhost:8019/memories/{id}/history
```

### AI Agent 드롭인 교체

```python
# mem0 (이전)
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem (이후 — HTTP로 전환만 하면 됨)
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 🏗️ 아키텍처

```
src/
├── main.rs          CLI 진입점 (clap)
├── config.rs        TOML + 환경 변수 설정
├── server.rs        REST API (axum)
├── memory.rs        코어 오케스트레이터 — 3계층 메모리 파이프라인
├── extract.rs       LLM prompts: 사실/엔티티/관계 추출
├── embedding.rs     OpenAI 호환 embedding 클라이언트
├── store.rs         SQLite vector store (cosine similarity)
└── graph.rs         SQLite graph store (soft-delete, 다중값 관계)
```

**8개 파일. 1,748줄. 외부 서비스 제로.**

---

## 🗺️ 로드맵

| 상태 | 기능 | 설명 |
|---|---|---|
| 🔲 | **MCP Server** | 메모리를 MCP tools로 Claude / Cursor에 제공 |
| 🔲 | **배치 임포트** | 기존 mem0 내보내기 데이터 로드 |
| 🔲 | **멀티모달** | 이미지/오디오 메모리 지원 |
| 🔲 | **Agent SDK** | Rust crate로 직접 임베딩 (HTTP 불필요) |
| 🔲 | **Dashboard** | 메모리 검사용 경량 web UI |

커뮤니티 기여를 환영합니다 — issue 또는 PR을 열어주세요.

---

<div align="center">

**MIT License**

[Ad Huang](https://github.com/Adaimade)이 [Claude Code](https://claude.ai)로 구축

</div>
