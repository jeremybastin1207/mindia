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
