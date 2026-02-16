//! Database transaction utilities
//!
//! Provides helpers for executing multiple database operations within a transaction
//! to ensure atomicity and data consistency.

use mindia_core::AppError;
use sqlx::{PgPool, Postgres, Transaction};
use std::pin::Pin;

/// Execute a closure within a database transaction
///
/// This function begins a transaction, executes the provided closure with the transaction,
/// and commits if successful or rolls back on error.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `f` - Closure that receives a transaction and returns a boxed future
///
/// # Returns
/// The result of the closure, or a database error if transaction management fails
pub async fn with_transaction<T, F>(pool: &PgPool, f: F) -> Result<T, AppError>
where
    F: for<'a> FnOnce(
        &'a mut Transaction<'_, Postgres>,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<T, AppError>> + Send + 'a>,
    >,
{
    let mut tx = pool.begin().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to begin transaction");
        AppError::Database(e)
    })?;

    match f(&mut tx).await {
        Ok(result) => {
            tx.commit().await.map_err(|e| {
                tracing::error!(error = %e, "Failed to commit transaction");
                AppError::Database(e)
            })?;
            Ok(result)
        }
        Err(e) => {
            if let Err(rollback_err) = tx.rollback().await {
                tracing::error!(
                    error = %rollback_err,
                    original_error = %e,
                    "Failed to rollback transaction"
                );
            }
            Err(e)
        }
    }
}

/// Execute a closure within a database transaction with automatic retry
///
/// This is similar to `with_transaction` but includes retry logic for transient errors.
/// Useful for operations that may fail due to temporary database issues.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `f` - Closure that receives a transaction and returns a boxed future
/// * `max_retries` - Maximum number of retry attempts (default: 3)
///
/// # Returns
/// The result of the closure, or a database error if all retries fail
#[allow(dead_code)]
pub async fn with_transaction_retry<T, F>(
    pool: &PgPool,
    f: F,
    max_retries: u32,
) -> Result<T, AppError>
where
    F: for<'a> Fn(
        &'a mut Transaction<'_, Postgres>,
    )
        -> Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send + 'a>>,
{
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match with_transaction(pool, |tx| f(tx)).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    let delay_ms = 100 * (attempt + 1) as u64; // Exponential backoff
                    tracing::warn!(
                        attempt = attempt + 1,
                        max_retries = max_retries,
                        delay_ms = delay_ms,
                        "Transaction failed, retrying"
                    );
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| AppError::Internal("Transaction failed after all retries".to_string())))
}
