//! Database transaction utilities
//!
//! This module provides utilities for working with database transactions,
//! particularly for multi-step operations that need atomicity.

use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, Transaction};
use std::ops::{Deref, DerefMut};

/// A database transaction wrapper for explicit commit/rollback.
///
/// **Important:** You must always call either [`commit`](TransactionGuard::commit) or
/// [`rollback`](TransactionGuard::rollback) before the guard is dropped. Drop cannot
/// perform an async rollback, so a guard dropped without commit/rollback leaves the
/// connection held until the guard is dropped; the transaction is not rolled back
/// automatically. In debug builds, dropping without commit/rollback will panic to
/// catch misuse.
///
/// Prefer [`with_transaction`] when possible; it handles commit/rollback for you.
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
        if self.transaction.is_some() {
            // We cannot perform async rollback in Drop. The connection is released
            // when the guard is dropped; the transaction remains open until the
            // connection is returned to the pool.
            #[cfg(debug_assertions)]
            panic!(
                "TransactionGuard dropped without commit or rollback. \
                 Always call .commit().await or .rollback().await explicitly, \
                 or use with_transaction() instead."
            );
            #[cfg(not(debug_assertions))]
            tracing::warn!(
                "TransactionGuard dropped without explicit commit or rollback; \
                 connection will be returned to pool without rollback. \
                 Prefer with_transaction() or always call commit/rollback."
            );
        }
    }
}

/// Execute a closure within a database transaction
///
/// Begins a transaction, runs the closure, then commits on success or rolls back
/// on error. Prefer this over manual `TransactionGuard` when the whole operation
/// fits in one async block, so you never risk dropping the guard without commit/rollback.
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
    E: Into<anyhow::Error> + Send + Sync + 'static,
{
    let mut tx = pool.begin().await.context("Failed to begin transaction")?;

    match f(&mut tx).await {
        Ok(result) => {
            tx.commit().await.context("Failed to commit transaction")?;
            Ok(result)
        }
        Err(e) => {
            tx.rollback().await.ok(); // Ignore rollback errors
            Err(e.into())
        }
    }
}
