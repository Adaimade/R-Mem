use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::config::AppConfig;
use crate::memory::MemoryManager;

struct AppState {
    memory: MemoryManager,
}

// ── Request / Response types ─────────────────────────────────────────

#[derive(Deserialize)]
struct AddRequest {
    user_id: String,
    text: String,
}

#[derive(Deserialize)]
struct SearchRequest {
    user_id: String,
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Deserialize)]
struct UpdateRequest {
    text: String,
}

#[derive(Deserialize)]
struct UserQuery {
    user_id: String,
}

#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

fn default_limit() -> usize {
    // Note: this is the serde default; runtime default comes from config.memory.api_search_limit
    100
}

#[derive(Serialize)]
struct GraphResponse {
    relations: Vec<crate::graph::Relation>,
}

// ── Routes ───────────────────────────────────────────────────────────

pub async fn run(config: AppConfig, memory: MemoryManager) -> anyhow::Result<()> {
    let state = Arc::new(AppState { memory });

    let app = Router::new()
        .route("/health", get(health))
        .route("/memories/add", post(add_memory))
        .route("/memories/search", post(search_memories))
        .route("/memories/{id}", get(get_memory))
        .route("/memories/{id}", put(update_memory))
        .route("/memories/{id}", delete(delete_memory))
        .route("/memories/{id}/history", get(memory_history))
        .route("/memories", get(get_all_memories))
        .route("/memories", delete(reset_memories))
        .route("/graph", get(get_graph))
        .route("/archive", get(get_archive))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.server.listen_addr()).await?;
    info!("Listening on {}", config.server.listen_addr());

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn add_memory(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddRequest>,
) -> Result<Json<ApiResponse<Vec<crate::memory::AddResult>>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.add(&req.user_id, &req.text).await {
        Ok(results) => Ok(Json(ApiResponse {
            success: true,
            data: results,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn search_memories(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<ApiResponse<Vec<crate::store::SearchResult>>>, (StatusCode, Json<ErrorResponse>)>
{
    match state.memory.search(&req.user_id, &req.query, req.limit).await {
        Ok(results) => Ok(Json(ApiResponse {
            success: true,
            data: results,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn get_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Option<crate::store::MemoryRecord>>>, (StatusCode, Json<ErrorResponse>)>
{
    match state.memory.get(&id).await {
        Ok(record) => Ok(Json(ApiResponse {
            success: true,
            data: record,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn update_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.update(&id, &req.text).await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: "updated".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn delete_memory(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.delete(&id).await {
        Ok(()) => Ok(Json(ApiResponse {
            success: true,
            data: "deleted".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn memory_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.history(&id).await {
        Ok(history) => Ok(Json(ApiResponse {
            success: true,
            data: history,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn get_all_memories(
    State(state): State<Arc<AppState>>,
    Query(q): Query<UserQuery>,
) -> Result<Json<ApiResponse<Vec<crate::store::MemoryRecord>>>, (StatusCode, Json<ErrorResponse>)>
{
    match state.memory.get_all(&q.user_id).await {
        Ok(records) => Ok(Json(ApiResponse {
            success: true,
            data: records,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn reset_memories(
    State(state): State<Arc<AppState>>,
    Query(q): Query<UserQuery>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.reset(&q.user_id).await {
        Ok(count) => Ok(Json(ApiResponse {
            success: true,
            data: format!("deleted {count} memories"),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn get_archive(
    State(state): State<Arc<AppState>>,
    Query(q): Query<UserQuery>,
) -> Result<Json<ApiResponse<Vec<crate::store::ArchivedRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.get_archive(&q.user_id).await {
        Ok(records) => Ok(Json(ApiResponse {
            success: true,
            data: records,
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}

async fn get_graph(
    State(state): State<Arc<AppState>>,
    Query(q): Query<UserQuery>,
) -> Result<Json<ApiResponse<GraphResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match state.memory.get_graph(&q.user_id).await {
        Ok(relations) => Ok(Json(ApiResponse {
            success: true,
            data: GraphResponse { relations },
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                success: false,
                error: e.to_string(),
            }),
        )),
    }
}
