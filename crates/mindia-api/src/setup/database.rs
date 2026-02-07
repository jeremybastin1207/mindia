//! Database setup and initialization

use anyhow::{Context, Result};
use mindia_core::Config;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::path::Path;
use std::time::Duration;

/// Setup database connection pool and run migrations
pub async fn setup_database(config: &Config) -> Result<PgPool> {
    tracing::info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections())
        .acquire_timeout(Duration::from_secs(config.db_timeout_seconds()))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(config.database_url())
        .await?;

    tracing::info!(
        max_connections = config.db_max_connections(),
        "Database connected successfully"
    );

    // Run pending migrations on startup (path: workspace migrations/ from crate root)
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let migrator = sqlx::migrate::Migrator::new(migrations_dir)
        .await
        .context("Failed to load migrations")?;
    migrator
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;
    tracing::info!("Database migrations applied");

    Ok(pool)
}
