pub mod config;
pub mod embedding;
pub mod extract;
pub mod graph;
pub mod memory;
pub mod store;

// Re-export key types for convenience
pub use config::AppConfig;
pub use graph::{GraphStore, Relation};
pub use memory::MemoryManager;
pub use store::{MemoryRecord, MemoryStore, SearchResult};
