🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

# R-Mem

**mem0의 Rust 구현. AI agent를 위한 장기 메모리. 단일 바이너리. Python 불필요.**

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| 바이너리 / Runtime | **3.2 MB** 정적 링크 | Python + pip 필요            |
| 유휴 메모리 (RSS)  | **< 10 MB**        | 200 MB+                      |
| 코드 라인 수       | **1,748**          | ~91,500                      |
| Vector Store      | SQLite (내장)       | Qdrant + 26종 이상            |
| Graph Store       | SQLite (내장)       | Neo4j / Memgraph             |
| 의존성             | 컴파일 시 포함       | pip install mem0ai           |
| LLM 백엔드        | 모든 OpenAI 호환 엔드포인트 (Ollama) | OpenAI / Anthropic만 |

---

## 왜 R-Mem인가

mem0는 강력합니다. 하지만 91,500줄의 Python 코드가 있고, 실행 중인 벡터 데이터베이스가 필요하며, 아무것도 하기 전에 200MB+ RAM을 소비합니다.

R-Mem은 동일한 3계층 메모리 아키텍처 — vector memory, graph memory, history — 를 1,748줄의 Rust로 구현합니다. SQLite가 벡터와 그래프 스토리지를 모두 처리합니다. 외부 서비스 불필요. 런타임 불필요. 바이너리 하나로 완결.

Claude Code로 완전히 구축.

> **참고:** 이 프로젝트는 AI 메모리 시스템을 Rust로 재구현하는 연구입니다. 핵심 로직과 아키텍처는 [mem0](https://github.com/mem0ai/mem0)를 참고했습니다. 토론, 아이디어, 기여를 환영합니다!

---

## 작동 방식

```
Input text
│
├── Vector Memory (장기 사실)
│     ├── LLM이 사실 추출 → ["Name is Alice", "Works at Google"]
│     ├── Embedding → cosine similarity 검색 (기존 상위 5개)
│     ├── Integer ID mapping (LLM UUID 할루시네이션 방지)
│     ├── LLM이 각 사실에 대해 판정:
│     │     ├── ADD       → 새로운 정보
│     │     ├── UPDATE    → 더 구체적 ("likes sports" → "likes tennis")
│     │     ├── DELETE    → 모순 ("likes pizza" vs "hates pizza")
│     │     └── NONE      → 중복, 건너뛰기
│     └── 실행 + 히스토리 기록
│
└── Graph Memory (엔티티 관계)
      ├── LLM이 엔티티 + 관계 추출
      ├── 충돌 감지 (기존 데이터 soft-delete, 새 데이터 추가)
      └── 다중값 vs 단일값 관계 처리
```

---

## 빠른 시작

### 사전 요구사항

- Rust 툴체인 (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- LLM 백엔드: [Ollama](https://ollama.com) (로컬) 또는 모든 OpenAI 호환 엔드포인트

### 빌드

```bash
git clone https://github.com/Adaimade/R-Mem.git && cd R-Mem
cargo build --release
# 바이너리: target/release/rustmem (3.2 MB)
```

### 설정

`rustmem.toml` 생성:

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

또는 OpenAI 사용:

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

## 사용법

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

## 아키텍처

```
src/
├── main.rs        # CLI (clap)
├── config.rs      # TOML + 환경 변수 설정
├── server.rs      # REST API (axum)
├── memory.rs      # 코어 오케스트레이터
├── extract.rs     # LLM prompts: 사실/엔티티/관계 추출
├── embedding.rs   # OpenAI 호환 embedding 클라이언트
├── store.rs       # SQLite vector store (cosine similarity)
└── graph.rs       # SQLite graph store (soft-delete, 다중값 관계)
```

---

## AI Agent와 통합

```python
# mem0 (이전)
from mem0 import Memory
m = Memory()
m.add("Alice loves sushi", user_id="alice")

# R-Mem (이후 — HTTP를 통한 드롭인 교체)
import httpx
httpx.post("http://localhost:8019/memories/add",
    json={"user_id": "alice", "text": "Alice loves sushi"})
```

---

## 로드맵

- [ ] MCP server — 메모리를 MCP tools로 Claude / Cursor에 제공
- [ ] 배치 임포트 — 기존 mem0 내보내기 데이터 로드
- [ ] 멀티모달 — 이미지/오디오 메모리 지원
- [ ] Agent SDK — Rust crate로 직접 임베딩 (HTTP 불필요)
- [ ] Dashboard — 메모리 검사용 경량 web UI

커뮤니티 기여를 환영합니다. issue 또는 PR을 열어주세요.

---

## 라이선스

MIT
