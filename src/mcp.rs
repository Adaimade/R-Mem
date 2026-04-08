use std::sync::Arc;

use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};

use crate::memory::MemoryManager;

// ── Parameter types ─────────────────────────────────────────────

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct AddMemoryParams {
    /// User ID to associate the memory with
    pub user_id: String,
    /// Text content to memorize (facts will be extracted automatically)
    pub text: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct SearchMemoryParams {
    /// User ID to search memories for
    pub user_id: String,
    /// Natural language search query
    pub query: String,
    /// Maximum number of results (default: 10)
    pub limit: Option<usize>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct UserIdParam {
    /// User ID
    pub user_id: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MemoryIdParam {
    /// Memory ID (UUID)
    pub memory_id: String,
}

// ── MCP Server ──────────────────────────────────────────────────

#[derive(Clone)]
pub struct RMemMcpServer {
    memory: Arc<MemoryManager>,
    tool_router: ToolRouter<Self>,
}

fn mcp_err(e: impl std::fmt::Display) -> McpError {
    McpError::internal_error(e.to_string(), None)
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<String, McpError> {
    serde_json::to_string_pretty(v).map_err(mcp_err)
}

#[tool_router]
impl RMemMcpServer {
    pub fn new(memory: MemoryManager) -> Self {
        Self {
            memory: Arc::new(memory),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Add a memory. Text is analyzed, facts extracted, deduplicated against existing memories, and stored. Returns the list of actions taken (ADD/UPDATE/DELETE).")]
    async fn add_memory(
        &self,
        Parameters(params): Parameters<AddMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let results = self.memory.add(&params.user_id, &params.text).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(to_json(&results)?)]))
    }

    #[tool(description = "Search memories by semantic similarity. Combines vector search and graph relations. Returns ranked results with relevance scores.")]
    async fn search_memory(
        &self,
        Parameters(params): Parameters<SearchMemoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(10);
        let results = self.memory.search(&params.user_id, &params.query, limit).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(to_json(&results)?)]))
    }

    #[tool(description = "List all stored memories for a user.")]
    async fn list_memories(
        &self,
        Parameters(params): Parameters<UserIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let records = self.memory.get_all(&params.user_id).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(to_json(&records)?)]))
    }

    #[tool(description = "Get a specific memory by its ID.")]
    async fn get_memory(
        &self,
        Parameters(params): Parameters<MemoryIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let record = self.memory.get(&params.memory_id).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(to_json(&record)?)]))
    }

    #[tool(description = "Delete a specific memory by its ID.")]
    async fn delete_memory(
        &self,
        Parameters(params): Parameters<MemoryIdParam>,
    ) -> Result<CallToolResult, McpError> {
        self.memory.delete(&params.memory_id).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text("Memory deleted.".to_string())]))
    }

    #[tool(description = "Get the knowledge graph (entity relationships) for a user. Returns all valid relations with source, relation type, and destination.")]
    async fn get_graph(
        &self,
        Parameters(params): Parameters<UserIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let relations = self.memory.get_graph(&params.user_id).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(to_json(&relations)?)]))
    }

    #[tool(description = "Delete ALL memories and graph data for a user. This is irreversible.")]
    async fn reset_memories(
        &self,
        Parameters(params): Parameters<UserIdParam>,
    ) -> Result<CallToolResult, McpError> {
        let count = self.memory.reset(&params.user_id).await.map_err(mcp_err)?;
        Ok(CallToolResult::success(vec![Content::text(
            format!("Deleted {count} memories and all graph data."),
        )]))
    }
}

#[tool_handler]
impl ServerHandler for RMemMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation {
                name: "rustmem-mcp".to_string(),
                title: Some("R-Mem".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: Some("https://github.com/Adaimade/R-Mem".to_string()),
            },
            instructions: Some(
                "R-Mem: Long-term memory for AI agents. \
                 Use add_memory to store facts from conversations, \
                 search_memory to retrieve relevant memories by semantic similarity, \
                 get_graph to explore entity relationships, \
                 and list_memories to see all stored facts for a user."
                    .to_string(),
            ),
        }
    }
}

/// Run the MCP server over stdio.
pub async fn run(memory: MemoryManager) -> anyhow::Result<()> {
    let service = RMemMcpServer::new(memory)
        .serve(stdio())
        .await?;
    service.waiting().await?;
    Ok(())
}
