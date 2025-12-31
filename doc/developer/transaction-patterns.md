# Database Transaction Patterns

## Overview

This document describes the transaction patterns used in Mindia for ensuring data consistency in multi-step operations.

## Transaction Utilities

The `mindia-db` crate provides transaction utilities in `mindia_db::db::transaction`:

- `TransactionGuard`: A wrapper that automatically handles commit/rollback
- `with_transaction`: A helper function for executing closures within transactions

## Usage Patterns

### Pattern 1: Upload Operations

Upload operations typically involve:
1. Upload file to storage (S3/local) - **cannot be transactional**
2. Save metadata to database - **can be transactional**
3. Queue background task - **may be transactional** (if using database-backed queue)

**Current Implementation**:
- Storage upload happens first
- If database save fails, storage is manually cleaned up
- Task queue submission happens after database save (best-effort)

**Recommended Pattern**:
```rust
use mindia_db::with_transaction;

// 1. Upload to storage (must happen first, can't be rolled back)
let (storage_key, storage_url) = storage.upload(...).await?;

// 2. Save metadata and queue task in a transaction
let result = with_transaction(&pool, |tx| async move {
    // Save metadata
    let image = image_repo.create_image_with_tx(&mut *tx, ...).await?;
    
    // Queue task (if using database-backed queue)
    task_repo.create_task_with_tx(&mut *tx, ...).await?;
    
    Ok(image)
}).await;

match result {
    Ok(image) => Ok(image),
    Err(e) => {
        // Cleanup storage on transaction failure
        storage.delete(&storage_key).await.ok();
        Err(e)
    }
}
```

### Pattern 2: Multi-Step Database Operations

For operations that only involve database changes:

```rust
use mindia_db::with_transaction;

let result = with_transaction(&pool, |tx| async move {
    // Step 1: Update media record
    media_repo.update_with_tx(&mut *tx, ...).await?;
    
    // Step 2: Update metadata
    metadata_repo.update_with_tx(&mut *tx, ...).await?;
    
    // Step 3: Log audit event
    audit_repo.create_with_tx(&mut *tx, ...).await?;
    
    Ok(())
}).await?;
```

### Pattern 3: Using TransactionGuard Directly

For more control over transaction lifecycle:

```rust
use mindia_db::TransactionGuard;

let mut tx = TransactionGuard::begin(&pool).await?;

// Perform operations
sqlx::query("INSERT INTO ...").execute(&mut *tx).await?;
sqlx::query("UPDATE ...").execute(&mut *tx).await?;

// Commit (or let it rollback on drop if not committed)
tx.commit().await?;
```

## Repository Methods with Transaction Support

Repositories should provide methods that accept a transaction parameter for use within transactions:

```rust
impl ImageRepository {
    // Standard method (uses pool directly)
    pub async fn create_image(&self, ...) -> Result<Image, AppError> {
        sqlx::query_as("INSERT INTO ...")
            .fetch_one(&self.pool)
            .await
    }
    
    // Transaction-aware method
    pub async fn create_image_with_tx(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        ...
    ) -> Result<Image, AppError> {
        sqlx::query_as("INSERT INTO ...")
            .fetch_one(&mut *tx)
            .await
    }
}
```

## Storage Operations

⚠️ **Important**: Storage operations (S3, local filesystem) **cannot be transactional**. They must be handled separately:

1. **Upload-then-save pattern**: Upload to storage first, then save to database. If database save fails, manually delete from storage.
2. **Save-then-upload pattern**: Save to database first, then upload to storage. If upload fails, delete from database.

The current codebase uses the **upload-then-save** pattern, which is generally safer as it avoids orphaned database records.

## Best Practices

1. **Use transactions for database operations**: All multi-step database operations should use transactions
2. **Handle storage cleanup**: Always clean up storage if database operations fail
3. **Keep transactions short**: Don't hold transactions open for long-running operations
4. **Use `with_transaction` for simple cases**: Prefer the helper function for straightforward transaction patterns
5. **Use `TransactionGuard` for complex cases**: When you need more control over transaction lifecycle

## Migration Guide

To migrate existing code to use transactions:

1. **Identify multi-step operations**: Look for operations that perform multiple database writes
2. **Add transaction-aware repository methods**: Create `*_with_tx` methods that accept a transaction
3. **Wrap operations in transactions**: Use `with_transaction` or `TransactionGuard`
4. **Handle storage cleanup**: Ensure storage is cleaned up if database operations fail

## Examples

See the following files for examples:
- `mindia-db/src/db/media/task.rs::claim_next_task()` - Example of transaction usage
- `mindia-db/src/db/control/webhook.rs::get_due_retries()` - Example of transaction with rollback

## Implementation Status

✅ **Transaction utilities created**: `TransactionGuard` and `with_transaction` helper are available in `mindia_db::db::transaction`

✅ **Example repository method**: `ImageRepository::create_image_with_tx()` demonstrates the pattern

⏳ **Migration in progress**: Upload handlers can be incrementally migrated to use transactions

## Future Improvements

1. **Repository trait with transaction support**: Create a trait that repositories can implement for transaction-aware methods
2. **Automatic storage cleanup**: Create a utility that automatically cleans up storage on transaction failure
3. **Two-phase commit for storage**: Investigate distributed transaction patterns for storage operations (complex, may not be worth it)
4. **Migrate all upload handlers**: Update image, video, audio, and document upload handlers to use transactions
