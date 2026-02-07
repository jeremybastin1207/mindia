mod api_doc;
mod auth;
mod constants;
mod error;
mod handlers;
mod http_metrics;
mod job_queue;
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

// ApiDoc moved to api_doc.rs

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load configuration
    let config = Config::from_env()?;

    // Initialize the application (database, services, routes)
    let (_state, router) = crate::setup::initialize_app(config.clone()).await?;

    // Start the server
    crate::setup::server::start_server(&config, router).await?;

    Ok(())
}
