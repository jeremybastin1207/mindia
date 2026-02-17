use chrono::{DateTime, Utc};
use mindia_core::AppError;
use sqlx::{PgPool, Row};
use uuid::Uuid;

/// Repository for managing presigned upload sessions
#[derive(Clone)]
pub struct PresignedUploadRepository {
    pool: PgPool,
}

impl PresignedUploadRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new presigned upload session
    #[allow(clippy::too_many_arguments)]
    pub async fn create_upload_session(
        &self,
        tenant_id: Uuid,
        upload_id: Uuid,
        filename: String,
        content_type: String,
        file_size: u64,
        media_type: String,
        s3_key: String,
        store_behavior: String,
        expires_at: DateTime<Utc>,
        metadata: Option<serde_json::Value>,
        chunk_size: Option<u64>,
        chunk_count: Option<i32>,
    ) -> Result<(), AppError> {
        // Use dynamic SQLx queries to avoid requiring DATABASE_URL/sqlx prepare
        sqlx::query(
            r#"
            INSERT INTO presigned_upload_sessions (
                id, tenant_id, filename, content_type, file_size, 
                media_type, s3_key, store_behavior, expires_at, metadata, status,
                chunk_size, chunk_count, uploaded_size
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending', $11, $12, 0)
            "#,
        )
        .bind(upload_id)
        .bind(tenant_id)
        .bind(filename)
        .bind(content_type)
        .bind(file_size as i64)
        .bind(media_type)
        .bind(s3_key)
        .bind(store_behavior)
        .bind(expires_at)
        .bind(metadata)
        .bind(chunk_size.map(|s| s as i64))
        .bind(chunk_count)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an upload session by ID
    pub async fn get_upload_session(
        &self,
        tenant_id: Uuid,
        upload_id: Uuid,
    ) -> Result<Option<UploadSession>, AppError> {
        let row = sqlx::query_as::<_, UploadSession>(
            r#"
            SELECT 
                id, tenant_id, filename, content_type, file_size, 
                media_type, s3_key, store_behavior, expires_at, 
                metadata, status::text as status, file_id, error_message, 
                chunk_size, chunk_count, COALESCE(uploaded_size, 0) as uploaded_size,
                created_at, updated_at
            FROM presigned_upload_sessions
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(upload_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Record a chunk upload
    pub async fn record_chunk(
        &self,
        session_id: Uuid,
        chunk_index: i32,
        s3_key: String,
        size: i64,
    ) -> Result<(), AppError> {
        // Insert chunk record
        sqlx::query(
            r#"
            INSERT INTO upload_chunks (session_id, chunk_index, s3_key, size)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (session_id, chunk_index) DO NOTHING
            "#,
        )
        .bind(session_id)
        .bind(chunk_index)
        .bind(s3_key)
        .bind(size)
        .execute(&self.pool)
        .await?;

        // Update uploaded_size in session
        sqlx::query(
            r#"
            UPDATE presigned_upload_sessions
            SET uploaded_size = uploaded_size + $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(session_id)
        .bind(size)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all chunks for a session
    pub async fn get_chunks(&self, session_id: Uuid) -> Result<Vec<UploadChunk>, AppError> {
        let chunks = sqlx::query_as::<_, UploadChunk>(
            r#"
            SELECT id, session_id, chunk_index, s3_key, size, uploaded_at
            FROM upload_chunks
            WHERE session_id = $1
            ORDER BY chunk_index
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(chunks)
    }

    /// Update session status
    pub async fn update_status(&self, session_id: Uuid, status: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE presigned_upload_sessions
            SET status = $2::upload_session_status, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(session_id)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark upload session as completed
    pub async fn mark_completed(&self, upload_id: Uuid, file_id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE presigned_upload_sessions
            SET status = 'completed', file_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(upload_id)
        .bind(file_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark upload session as failed
    pub async fn mark_failed(
        &self,
        upload_id: Uuid,
        error_message: Option<String>,
    ) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE presigned_upload_sessions
            SET status = 'failed', error_message = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(upload_id)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Clean up expired upload sessions
    pub async fn cleanup_expired(&self) -> Result<u64, AppError> {
        let result = sqlx::query(
            r#"
            DELETE FROM presigned_upload_sessions
            WHERE expires_at < NOW() AND status = 'pending'
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

/// Upload session record
#[derive(Debug)]
pub struct UploadSession {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub media_type: String,
    pub s3_key: String,
    pub store_behavior: String,
    pub expires_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
    pub status: String,
    pub file_id: Option<Uuid>,
    pub error_message: Option<String>,
    pub chunk_size: Option<i64>,
    pub chunk_count: Option<i32>,
    pub uploaded_size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for UploadSession {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        Ok(UploadSession {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            filename: row.get("filename"),
            content_type: row.get("content_type"),
            file_size: row.get("file_size"),
            media_type: row.get("media_type"),
            s3_key: row.get("s3_key"),
            store_behavior: row.get("store_behavior"),
            expires_at: row.get("expires_at"),
            metadata: row.get("metadata"),
            status: row.get("status"),
            file_id: row.get("file_id"),
            error_message: row.get("error_message"),
            chunk_size: row.get("chunk_size"),
            chunk_count: row.get("chunk_count"),
            uploaded_size: row.get("uploaded_size"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }
}

/// Upload chunk record
#[derive(Debug, sqlx::FromRow)]
pub struct UploadChunk {
    pub id: Uuid,
    pub session_id: Uuid,
    pub chunk_index: i32,
    pub s3_key: String,
    pub size: i64,
    pub uploaded_at: DateTime<Utc>,
}
