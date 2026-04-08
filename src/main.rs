mod config;
mod embedding;
mod extract;
mod graph;
mod memory;
mod server;
mod store;

use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "rustmem", about = "Long-term memory for AI agents", version)]
struct Cli {
    #[arg(short, long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Start the memory API server
    Server,
    /// Add a memory from the command line
    Add {
        #[arg(short, long)]
        user: String,
        text: String,
    },
    /// Search memories
    Search {
        #[arg(short, long)]
        user: String,
        query: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// List all memories for a user
    List {
        #[arg(short, long)]
        user: String,
    },
    /// Show graph relations for a user
    Graph {
        #[arg(short, long)]
        user: String,
    },
    /// Reset all memories for a user
    Reset {
        #[arg(short, long)]
        user: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rustmem=info".parse()?),
        )
        .init();

    let cli = Cli::parse();
    let cfg = config::AppConfig::load(cli.config.as_deref())?;

    match cli.command {
        Command::Server => {
            info!("Starting rustmem server on {}", cfg.server.listen_addr());
            let mem = memory::MemoryManager::new(&cfg).await?;
            server::run(cfg, mem).await
        }
        Command::Add { user, text } => {
            let mem = memory::MemoryManager::new(&cfg).await?;
            let results = mem.add(&user, &text).await?;
            for r in &results {
                println!("[{}] {} → {}", r.event, r.id, r.text);
            }
            if results.is_empty() {
                println!("No facts extracted.");
            }
            Ok(())
        }
        Command::Search { user, query, limit } => {
            let mem = memory::MemoryManager::new(&cfg).await?;
            let results = mem.search(&user, &query, limit).await?;
            if results.is_empty() {
                println!("No memories found.");
            }
            for r in &results {
                println!("[{:.3}] {} — {}", r.score, r.id, r.text);
            }
            Ok(())
        }
        Command::List { user } => {
            let mem = memory::MemoryManager::new(&cfg).await?;
            let records = mem.get_all(&user).await?;
            if records.is_empty() {
                println!("No memories for user '{user}'.");
            }
            for r in &records {
                println!("{} | {} | {}", r.id, r.updated_at, r.text);
            }
            Ok(())
        }
        Command::Graph { user } => {
            let mem = memory::MemoryManager::new(&cfg).await?;
            let g = graph::GraphStore::open(&cfg.store.db_path)?;
            let relations = g.get_all(&user).await?;
            if relations.is_empty() {
                println!("No graph relations for user '{user}'.");
            }
            for r in &relations {
                println!(
                    "{} --[{}]--> {} (mentions: {})",
                    r.source, r.relation, r.destination, r.mentions
                );
            }
            Ok(())
        }
        Command::Reset { user } => {
            let mem = memory::MemoryManager::new(&cfg).await?;
            let count = mem.reset(&user).await?;
            println!("Deleted {count} memories for user '{user}'.");
            Ok(())
        }
    }
}
