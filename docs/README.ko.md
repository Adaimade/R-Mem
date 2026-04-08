🌐 [English](../README.md) · [繁體中文](README.zh-TW.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

# R-Mem

**[mem0](https://github.com/mem0ai/mem0)의 메모리 아키텍처를 Rust로 연구하는 경량 구현. AI agent를 위한 장기 메모리. 단일 바이너리. Python 불필요.**

> 이 프로젝트는 학습 목적으로 [mem0](https://github.com/mem0ai/mem0)의 우아한 메모리 아키텍처를 Rust로 재구현한 것입니다. 원본 설계에 대한 공은 전적으로 mem0 팀에게 있습니다. 이것은 대체품이 아니라 다른 언어로 접근하는 아키텍처 연구입니다. 토론, 아이디어, 기여를 환영합니다!

아래 표는 의도적인 트레이드오프를 반영합니다 — mem0의 풍부한 에코시스템은 더 많은 유연성과 통합을 제공하며, R-Mem은 최소한의 풋프린트를 위해 이를 의도적으로 희생합니다.

|                   | **R-Mem**          | **mem0**                     |
|-------------------|--------------------|------------------------------|
| 바이너리 / Runtime | 3.2 MB 정적 링크    | Python + pip (풍부한 에코시스템)|
| 유휴 메모리 (RSS)  | < 10 MB            | 200 MB+ (더 많은 기능 로드)    |
| 코드 라인 수       | 1,748              | ~91,500 (26+ 종 store 지원)   |
| Vector Store      | SQLite만            | Qdrant, Chroma, Pinecone 등   |
| Graph Store       | SQLite만            | Neo4j / Memgraph             |
| 의존성             | 컴파일 시 포함       | pip install mem0ai           |
| LLM 백엔드        | 모든 OpenAI 호환 엔드포인트 (Ollama) | OpenAI, Anthropic 등 |

---

## 왜 R-Mem인가

mem0는 잘 설계된 메모리 시스템으로 풍부한 plugin 에코시스템을 가지고 있습니다. R-Mem은 더 좁은 질문을 던집니다: *핵심 메모리 로직만 Rust로 다시 작성하고, 완전히 SQLite를 백엔드로 사용하면 어떻게 될까?*

결과는 동일한 3계층 아키텍처 — vector memory, graph memory, history — 를 1,748줄의 Rust로 구현. 외부 서비스 불필요. 바이너리 하나. 트레이드오프는 명확: 통합 수는 mem0보다 훨씬 적지만, 운영 오버헤드는 거의 제로.

Claude Code로 구축.

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
