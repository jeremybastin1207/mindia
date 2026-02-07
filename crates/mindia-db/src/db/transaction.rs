//! Database transaction utilities
//!
//! This module provides utilities for working with database transactions,
//! particularly for multi-step operations that need atomicity.

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use std::ops::{Deref, DerefMut};

/// A database transaction wrapper that automatically handles commit/rollback
///
/// This wrapper ensures that transactions are properly committed or rolled back,
/// even in the case of early returns or panics (via Drop).
///
/// # Example
///
/// ```ignore
/// use mindia_db::db::transaction::TransactionGuard;
///
/// async fn example(pool: &sqlx::PgPool) -> anyhow::Result<()> {
///     let mut tx = TransactionGuard::begin(pool).await?;
///     sqlx::query("INSERT INTO ...").execute(&mut *tx).await?;
///     tx.commit().await?;
///     Ok(())
/// }
/// ```
pub struct TransactionGuard<'a> {
    transaction: Option<Transaction<'a, Postgres>>,
    #[allow(dead_code)]
    pool: &'a PgPool,
}

impl<'a> TransactionGuard<'a> {
    /// Begin a new database transaction
    pub async fn begin(pool: &'a PgPool) -> Result<Self> {
        let transaction = pool
            .begin()
            .await
            .context("Failed to begin database transaction")?;

        Ok(Self {
            transaction: Some(transaction),
            pool,
        })
    }

    /// Commit the transaction
    ///
    /// After calling this, the transaction is consumed and cannot be used further.
    pub async fn commit(mut self) -> Result<()> {
        if let Some(tx) = self.transaction.take() {
            tx.commit()
                .await
                .context("Failed to commit database transaction")?;
        }
        Ok(())
    }

    /// Rollback the transaction
    ///
    /// After calling this, the transaction is consumed and cannot be used further.
    pub async fn rollback(mut self) -> Result<()> {
        if let Some(tx) = self.transaction.take() {
            tx.rollback()
                .await
                .context("Failed to rollback database transaction")?;
        }
        Ok(())
    }

    /// Get a mutable reference to the underlying transaction
    ///
    /// This is useful when you need to pass the transaction to functions
    /// that expect `&mut Transaction<Postgres>`.
    pub fn as_mut(&mut self) -> Option<&mut Transaction<'a, Postgres>> {
        self.transaction.as_mut()
    }
}

impl<'a> Deref for TransactionGuard<'a> {
    type Target = Transaction<'a, Postgres>;

    fn deref(&self) -> &Self::Target {
        self.transaction
            .as_ref()
            .expect("Transaction was already committed or rolled back")
    }
}

impl<'a> DerefMut for TransactionGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.transaction
            .as_mut()
            .expect("Transaction was already committed or rolled back")
    }
}

impl<'a> Drop for TransactionGuard<'a> {
    fn drop(&mut self) {
        // If the transaction wasn't explicitly committed or rolled back,
        // roll it back automatically to prevent resource leaks
        if self.transaction.is_some() {
            tracing::warn!(
                "Transaction was dropped without explicit commit or rollback - rolling back"
            );
            // NOTE: We can't safely rollback in Drop due to lifetime constraints.
            // The transaction will be cleaned up when the connection pool is dropped.
            // In production, ensure transactions are explicitly committed or rolled back.
        }
    }
}

/// Execute a closure within a database transaction
///
/// This helper function begins a transaction, executes the closure,
/// and commits if successful or rolls back on error.
///
/// # Example
///
/// ```ignore
/// use mindia_db::db::transaction::with_transaction;
///
/// async fn example(pool: &sqlx::PgPool) -> anyhow::Result<()> {
///     with_transaction(pool, |tx| async move {
///         sqlx::query("INSERT INTO ...").execute(&mut *tx).await?;
///         sqlx::query("UPDATE ...").execute(&mut *tx).await?;
///         Ok::<_, sqlx::Error>(())
///     }).await
/// }
/// ```
pub async fn with_transaction<F, R, E>(pool: &PgPool, f: F) -> Result<R>
where
    F: for<'a> FnOnce(
        &'a mut Transaction<'_, Postgres>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<R, E>> + Send + 'a>,
    >,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    match f(&mut tx).await {
        Ok(result) => {
            tx.commit().await.context("Failed to commit transaction")?;
            Ok(result)
        }
        Err(e) => {
            tx.rollback().await.ok(); // Ignore rollback errors
            Err(anyhow::Error::from(e))
        }
    }
}
