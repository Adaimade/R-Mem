use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub embedding: EmbeddingConfig,
    #[serde(default)]
    pub store: StoreConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemoryConfig {
    /// Number of similar memories to retrieve for dedup (default: 5)
    #[serde(default = "default_search_top_k")]
    pub search_top_k: usize,
    /// Score assigned to graph matches in combined search (default: 0.9)
    #[serde(default = "default_graph_match_score")]
    pub graph_match_score: f32,
    /// Max relations returned from graph search (default: 20)
    #[serde(default = "default_graph_search_limit")]
    pub graph_search_limit: usize,
    /// Default search limit for API responses (default: 100)
    #[serde(default = "default_api_search_limit")]
    pub api_search_limit: usize,
    /// Minimum best score from active search before falling back to archive (default: 0.4)
    #[serde(default = "default_archive_fallback_threshold")]
    pub archive_fallback_threshold: f32,
    /// Max archive entries to keep per user before compaction (default: 200)
    #[serde(default = "default_archive_max_entries")]
    pub archive_max_entries: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            search_top_k: default_search_top_k(),
            graph_match_score: default_graph_match_score(),
            graph_search_limit: default_graph_search_limit(),
            api_search_limit: default_api_search_limit(),
            archive_fallback_threshold: default_archive_fallback_threshold(),
            archive_max_entries: default_archive_max_entries(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_bind")]
    pub bind: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            bind: default_bind(),
        }
    }
}

impl ServerConfig {
    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.bind, self.port)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: String::new(),
            base_url: String::new(),
            model: String::new(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmbeddingConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_embed_model")]
    pub model: String,
    #[serde(default = "default_dimensions")]
    pub dimensions: usize,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: String::new(),
            base_url: String::new(),
            model: default_embed_model(),
            dimensions: default_dimensions(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct StoreConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
        }
    }
}

fn default_port() -> u16 {
    8019
}
fn default_bind() -> String {
    "127.0.0.1".to_string()
}
fn default_provider() -> String {
    "openai".to_string()
}
fn default_embed_model() -> String {
    "nomic-embed-text".to_string()
}
fn default_dimensions() -> usize {
    768
}
fn default_db_path() -> String {
    "rustmem.db".to_string()
}
fn default_search_top_k() -> usize {
    5
}
fn default_graph_match_score() -> f32 {
    0.9
}
fn default_graph_search_limit() -> usize {
    20
}
fn default_api_search_limit() -> usize {
    100
}
fn default_archive_fallback_threshold() -> f32 {
    0.4
}
fn default_archive_max_entries() -> usize {
    200
}

impl AppConfig {
    pub fn load(cli_path: Option<&str>) -> anyhow::Result<Self> {
        let path = resolve_path(cli_path);

        let mut builder = config::Config::builder();

        if path.exists() {
            builder = builder.add_source(config::File::from(path.as_ref()).required(false));
        }

        builder = builder.add_source(
            config::Environment::with_prefix("RUSTMEM").separator("__"),
        );

        // Bare env vars
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            builder = builder.set_override("llm.api_key", key.clone())?;
            builder = builder.set_override("embedding.api_key", key)?;
        }

        let settings = builder.build()?;
        let cfg: AppConfig = settings.try_deserialize().unwrap_or_else(|e| {
            tracing::warn!(%e, "Config deserialization failed, using defaults");
            AppConfig::default()
        });
        Ok(cfg)
    }
}

fn resolve_path(cli_path: Option<&str>) -> PathBuf {
    if let Some(p) = cli_path {
        return PathBuf::from(p);
    }
    if let Ok(p) = std::env::var("RUSTMEM_CONFIG") {
        return PathBuf::from(p);
    }
    PathBuf::from("rustmem.toml")
}
