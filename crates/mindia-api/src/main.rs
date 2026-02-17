mod api_doc;
mod auth;
mod constants;
mod error;
mod handlers;
mod http_metrics;
mod job_queue;
mod landlock;
mod middleware;
#[cfg(feature = "plugin")]
mod plugins;
mod services;
mod setup;
mod state;
mod task_dispatch;
mod task_handlers;
mod telemetry;
mod utils;
mod validation;
mod video_storage_impl;

use mindia_core::Config;

// Use mimalloc as the global allocator for better performance and lower fragmentation,
// especially when running on musl-based systems inside containers.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// ApiDoc moved to api_doc.rs

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load .env before Landlock; once sandboxed we can only read under /app.
    mindia_core::config::load_dotenv();

    // Print DATABASE_URL before Landlock so environment access is unrestricted.
    // This reads the raw environment variable, which may differ from any
    // per-environment overrides you configure elsewhere.
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "<not set>".to_string());
    println!("DATABASE_URL={}", db_url);

    // Best-effort Landlock sandboxing on Linux.
    landlock::linux::init();

    // Load configuration
    let config = Config::from_env()?;

    // Initialize the application (database, services, routes)
    let (_state, router) = crate::setup::initialize_app(config.clone()).await?;

    // Start the server
    crate::setup::server::start_server(&config, router).await?;

    Ok(())
}
