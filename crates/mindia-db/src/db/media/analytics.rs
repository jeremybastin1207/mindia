use chrono::{DateTime, Utc};
use mindia_core::models::{
    AuditLogQuery, ContentTypeStats, RequestLog, RequestLogInput, StorageMetrics, UrlStatistics,
};
use mindia_core::AppError;
use sqlx::{PgPool, Postgres, Row};
use std::collections::HashMap;
use uuid::Uuid;

/// Trait for analytics repository operations
/// This abstracts the database implementation (PostgreSQL)
#[async_trait::async_trait]
pub trait AnalyticsRepositoryTrait: Send + Sync {
    async fn log_request(&self, log: RequestLogInput) -> Result<(), AppError>;

    async fn get_url_statistics(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<UrlStatistics>, AppError>;

    async fn get_traffic_summary(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64, i64, f64), AppError>;

    async fn get_requests_per_method(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<HashMap<String, i64>, AppError>;

    async fn get_requests_per_status(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<HashMap<i32, i64>, AppError>;

    async fn list_audit_logs(
        &self,
        tenant_id: Option<Uuid>,
        query: AuditLogQuery,
    ) -> Result<Vec<RequestLog>, AppError>;

    async fn count_audit_logs(
        &self,
        tenant_id: Option<Uuid>,
        query: &AuditLogQuery,
    ) -> Result<i64, AppError>;

    async fn get_audit_log_by_id(
        &self,
        id: i64,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<RequestLog>, AppError>;
}

/// Storage metrics repository - always uses PostgreSQL
/// since it queries the main application tables
#[derive(Clone)]
pub struct StorageMetricsRepository {
    pool: PgPool,
}

impl StorageMetricsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct PostgresAnalyticsRepository {
    pool: PgPool,
}

impl PostgresAnalyticsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AnalyticsRepositoryTrait for PostgresAnalyticsRepository {
    #[tracing::instrument(skip(self, log), fields(
        db.system = "postgresql",
        db.name = "mindia",
        db.table = "request_logs",
        db.operation = "insert",
        db.sql.table = "request_logs"
    ))]
    async fn log_request(&self, log: RequestLogInput) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO request_logs (
                tenant_id, method, path, normalized_path, query_string, status_code,
                request_size_bytes, response_size_bytes, duration_ms,
                user_agent, ip_address
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11::inet)
            "#,
        )
        .bind(log.tenant_id)
        .bind(&log.method)
        .bind(&log.path)
        .bind(&log.normalized_path)
        .bind(&log.query_string)
        .bind(log.status_code)
        .bind(log.request_size_bytes)
        .bind(log.response_size_bytes)
        .bind(log.duration_ms)
        .bind(&log.user_agent)
        .bind(&log.ip_address)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(
                error = ?e,
                method = %log.method,
                path = %log.path,
                ip_address = ?log.ip_address,
                "Failed to insert request log"
            );
            anyhow::anyhow!("PostgreSQL error inserting request log: {}", e)
        })?;

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(
        db.system = "postgresql",
        db.name = "mindia",
        db.table = "hourly_url_statistics",
        db.operation = "aggregate",
        db.sql.table = "hourly_url_statistics"
    ))]
    async fn get_url_statistics(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<UrlStatistics>, AppError> {
        let mut query = String::from(
            r#"
            SELECT 
                normalized_path as path,
                SUM(request_count)::BIGINT as request_count,
                SUM(total_bytes_sent)::BIGINT as total_bytes_sent,
                SUM(total_bytes_received)::BIGINT as total_bytes_received,
                CASE 
                    WHEN SUM(request_count) > 0 
                    THEN (SUM(total_duration_ms)::DOUBLE PRECISION / SUM(request_count)::DOUBLE PRECISION)
                    ELSE 0
                END as avg_response_time_ms,
                COALESCE(MIN(min_duration_ms), 0)::DOUBLE PRECISION as min_duration_ms,
                COALESCE(MAX(max_duration_ms), 0)::DOUBLE PRECISION as max_duration_ms,
                SUM(status_2xx)::BIGINT as status_2xx,
                SUM(status_3xx)::BIGINT as status_3xx,
                SUM(status_4xx)::BIGINT as status_4xx,
                SUM(status_5xx)::BIGINT as status_5xx
            FROM hourly_url_statistics
            "#,
        );

        let mut conditions: Vec<String> = Vec::new();
        if start_date.is_some() {
            conditions.push("bucket >= $1".to_string());
        }
        if end_date.is_some() {
            let param_num = if start_date.is_some() { 2 } else { 1 };
            conditions.push(format!("bucket <= ${}", param_num));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        let limit_param_num = match (start_date.is_some(), end_date.is_some()) {
            (true, true) => 3,
            (true, false) | (false, true) => 2,
            (false, false) => 1,
        };
        query.push_str(&format!(
            r#"
            GROUP BY normalized_path
            ORDER BY request_count DESC
            LIMIT ${}
            "#,
            limit_param_num
        ));

        let mut query_builder = sqlx::query_as::<Postgres, UrlStatistics>(&query);

        if let Some(start) = start_date {
            query_builder = query_builder.bind(start);
        }
        if let Some(end) = end_date {
            query_builder = query_builder.bind(end);
        }
        query_builder = query_builder.bind(limit);

        query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::from)
    }

    /// Get traffic summary aggregated across all tenants
    ///
    /// # Security Notice
    ///
    /// **This is a system-level operation** that queries across ALL tenants.
    /// It returns aggregated traffic statistics from all tenants combined.
    ///
    /// **Access Control:**
    /// - This method should ONLY be accessible to system administrators
    /// - User-facing handlers MUST NOT call this method directly
    ///
    /// **Use Cases:**
    /// - System-wide traffic monitoring
    /// - Infrastructure capacity planning
    /// - Global analytics dashboard (admin only)
    #[tracing::instrument(skip(self), fields(db.table = "hourly_traffic_summary", db.operation = "aggregate"))]
    async fn get_traffic_summary(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<(i64, i64, i64, f64), AppError> {
        let mut query = String::from(
            r#"
            SELECT 
                COALESCE(SUM(total_requests), 0)::BIGINT as total_requests,
                COALESCE(SUM(total_bytes_sent), 0)::BIGINT as total_bytes_sent,
                COALESCE(SUM(total_bytes_received), 0)::BIGINT as total_bytes_received,
                CASE 
                    WHEN SUM(total_requests) > 0 
                    THEN (SUM(total_duration_ms)::DOUBLE PRECISION / SUM(total_requests)::DOUBLE PRECISION)
                    ELSE 0
                END as avg_response_time_ms
            FROM hourly_traffic_summary
            "#,
        );

        let mut conditions: Vec<String> = Vec::new();
        if start_date.is_some() {
            conditions.push("bucket >= $1".to_string());
        }
        if end_date.is_some() {
            let param_num = if start_date.is_some() { 2 } else { 1 };
            conditions.push(format!("bucket <= ${}", param_num));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        let mut query_builder = sqlx::query(&query);

        if let Some(start) = start_date {
            query_builder = query_builder.bind(start);
        }
        if let Some(end) = end_date {
            query_builder = query_builder.bind(end);
        }

        let row = query_builder.fetch_one(&self.pool).await?;

        Ok((
            row.get::<i64, _>("total_requests"),
            row.get::<i64, _>("total_bytes_sent"),
            row.get::<i64, _>("total_bytes_received"),
            row.get::<f64, _>("avg_response_time_ms"),
        ))
    }

    #[tracing::instrument(skip(self), fields(db.table = "hourly_requests_by_method", db.operation = "aggregate"))]
    async fn get_requests_per_method(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<HashMap<String, i64>, AppError> {
        let mut query = String::from(
            r#"
            SELECT method, SUM(request_count)::BIGINT as count
            FROM hourly_requests_by_method
            "#,
        );

        let mut conditions: Vec<String> = Vec::new();
        if start_date.is_some() {
            conditions.push("bucket >= $1".to_string());
        }
        if end_date.is_some() {
            let param_num = if start_date.is_some() { 2 } else { 1 };
            conditions.push(format!("bucket <= ${}", param_num));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" GROUP BY method");

        let mut query_builder = sqlx::query(&query);

        if let Some(start) = start_date {
            query_builder = query_builder.bind(start);
        }
        if let Some(end) = end_date {
            query_builder = query_builder.bind(end);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut result = HashMap::new();
        for row in rows {
            let method: String = row.get("method");
            let count: i64 = row.get("count");
            result.insert(method, count);
        }

        Ok(result)
    }

    /// Get requests per HTTP status code aggregated across all tenants
    ///
    /// # Security Notice
    ///
    /// **This is a system-level operation** that queries across ALL tenants.
    /// It returns aggregated statistics from all tenants combined.
    ///
    /// **Access Control:**
    /// - This method should ONLY be accessible to system administrators
    /// - User-facing handlers MUST NOT call this method directly
    #[tracing::instrument(skip(self), fields(db.table = "hourly_requests_by_status", db.operation = "aggregate"))]
    async fn get_requests_per_status(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
    ) -> Result<HashMap<i32, i64>, AppError> {
        let mut query = String::from(
            r#"
            SELECT status_code, SUM(request_count)::BIGINT as count
            FROM hourly_requests_by_status
            "#,
        );

        let mut conditions: Vec<String> = Vec::new();
        if start_date.is_some() {
            conditions.push("bucket >= $1".to_string());
        }
        if end_date.is_some() {
            let param_num = if start_date.is_some() { 2 } else { 1 };
            conditions.push(format!("bucket <= ${}", param_num));
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" GROUP BY status_code");

        let mut query_builder = sqlx::query(&query);

        if let Some(start) = start_date {
            query_builder = query_builder.bind(start);
        }
        if let Some(end) = end_date {
            query_builder = query_builder.bind(end);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut result = HashMap::new();
        for row in rows {
            let status_code: i32 = row.get("status_code");
            let count: i64 = row.get("count");
            result.insert(status_code, count);
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self), fields(db.table = "request_logs", db.operation = "select_list"))]
    async fn list_audit_logs(
        &self,
        tenant_id: Option<Uuid>,
        query: AuditLogQuery,
    ) -> Result<Vec<RequestLog>, AppError> {
        let mut sql = String::from(
            r#"
            SELECT 
                id, request_id, tenant_id, method, path, normalized_path, 
                query_string, status_code, request_size_bytes, response_size_bytes,
                duration_ms, user_agent, ip_address, created_at
            FROM request_logs
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();
        let mut param_count = 1;

        // Add tenant_id filter if provided
        if tenant_id.is_some() {
            conditions.push(format!("AND tenant_id = ${}", param_count));
            param_count += 1;
        }

        // Add date range filters
        if query.start_date.is_some() {
            conditions.push(format!("AND created_at >= ${}", param_count));
            param_count += 1;
        }

        if query.end_date.is_some() {
            conditions.push(format!("AND created_at <= ${}", param_count));
            param_count += 1;
        }

        // Add method filter
        if query.method.is_some() {
            conditions.push(format!("AND method = ${}", param_count));
            param_count += 1;
        }

        // Add status code filter
        if query.status_code.is_some() {
            conditions.push(format!("AND status_code = ${}", param_count));
            param_count += 1;
        }

        // Add path filter (LIKE search)
        if query.path_filter.is_some() {
            conditions.push(format!(
                "AND (path ILIKE ${} OR normalized_path ILIKE ${})",
                param_count, param_count
            ));
            param_count += 1;
        }

        // Append conditions to SQL
        for condition in conditions {
            sql.push_str(&format!(" {}", condition));
        }

        // Add ordering and pagination
        sql.push_str(" ORDER BY created_at DESC");

        let limit = query.limit.unwrap_or(50).min(500); // Cap at 500
        sql.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;

        let offset = query.offset.unwrap_or(0);
        sql.push_str(&format!(" OFFSET ${}", param_count));

        // Build query with bindings
        let mut query_builder = sqlx::query_as::<Postgres, RequestLog>(&sql);

        // Bind parameters in the same order they were added
        if let Some(tid) = tenant_id {
            query_builder = query_builder.bind(tid);
        }

        if let Some(start) = query.start_date {
            query_builder = query_builder.bind(start);
        }

        if let Some(end) = query.end_date {
            query_builder = query_builder.bind(end);
        }

        if let Some(method) = &query.method {
            query_builder = query_builder.bind(method);
        }

        if let Some(status) = query.status_code {
            query_builder = query_builder.bind(status);
        }

        if let Some(path) = &query.path_filter {
            let pattern = format!("%{}%", path);
            query_builder = query_builder.bind(pattern);
        }

        query_builder = query_builder.bind(limit);
        query_builder = query_builder.bind(offset);

        query_builder
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::from)
    }

    #[tracing::instrument(skip(self), fields(db.table = "request_logs", db.operation = "count"))]
    async fn count_audit_logs(
        &self,
        tenant_id: Option<Uuid>,
        query: &AuditLogQuery,
    ) -> Result<i64, AppError> {
        let mut sql = String::from(
            r#"
            SELECT COUNT(*) as count
            FROM request_logs
            WHERE 1=1
            "#,
        );

        let mut conditions = Vec::new();
        let mut param_count = 1;

        // Add tenant_id filter if provided
        if tenant_id.is_some() {
            conditions.push(format!("AND tenant_id = ${}", param_count));
            param_count += 1;
        }

        // Add date range filters
        if query.start_date.is_some() {
            conditions.push(format!("AND created_at >= ${}", param_count));
            param_count += 1;
        }

        if query.end_date.is_some() {
            conditions.push(format!("AND created_at <= ${}", param_count));
            param_count += 1;
        }

        // Add method filter
        if query.method.is_some() {
            conditions.push(format!("AND method = ${}", param_count));
            param_count += 1;
        }

        // Add status code filter
        if query.status_code.is_some() {
            conditions.push(format!("AND status_code = ${}", param_count));
            param_count += 1;
        }

        // Add path filter (LIKE search)
        if query.path_filter.is_some() {
            conditions.push(format!(
                "AND (path ILIKE ${} OR normalized_path ILIKE ${})",
                param_count, param_count
            ));
            let _ = param_count;
        }

        // Append conditions to SQL
        for condition in conditions {
            sql.push_str(&format!(" {}", condition));
        }

        // Build query with bindings
        let mut query_builder = sqlx::query(&sql);

        // Bind parameters in the same order they were added
        if let Some(tid) = tenant_id {
            query_builder = query_builder.bind(tid);
        }

        if let Some(start) = query.start_date {
            query_builder = query_builder.bind(start);
        }

        if let Some(end) = query.end_date {
            query_builder = query_builder.bind(end);
        }

        if let Some(method) = &query.method {
            query_builder = query_builder.bind(method);
        }

        if let Some(status) = query.status_code {
            query_builder = query_builder.bind(status);
        }

        if let Some(path) = &query.path_filter {
            let pattern = format!("%{}%", path);
            query_builder = query_builder.bind(pattern);
        }

        let row = query_builder.fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("count"))
    }

    #[tracing::instrument(skip(self), fields(db.table = "request_logs", db.operation = "select_one"))]
    async fn get_audit_log_by_id(
        &self,
        id: i64,
        tenant_id: Option<Uuid>,
    ) -> Result<Option<RequestLog>, AppError> {
        let mut sql = String::from(
            r#"
            SELECT 
                id, request_id, tenant_id, method, path, normalized_path, 
                query_string, status_code, request_size_bytes, response_size_bytes,
                duration_ms, user_agent, ip_address, created_at
            FROM request_logs
            WHERE id = $1
            "#,
        );

        let mut query_builder = sqlx::query_as::<Postgres, RequestLog>(&sql);
        query_builder = query_builder.bind(id);

        // Add tenant isolation if tenant_id is provided
        if let Some(tid) = tenant_id {
            sql.push_str(" AND tenant_id = $2");
            query_builder = sqlx::query_as::<Postgres, RequestLog>(&sql);
            query_builder = query_builder.bind(id).bind(tid);
        }

        query_builder
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::from)
    }
}

impl StorageMetricsRepository {
    /// Calculate and store storage metrics aggregated across all tenants
    ///
    /// # Security Notice
    ///
    /// **This is a system-level operation** that queries across ALL tenants.
    /// It calculates total storage usage and file counts from all tenants combined.
    ///
    /// **Access Control:**
    /// - This method should ONLY be accessible to system administrators
    /// - User-facing handlers MUST NOT call this method directly
    /// - Typically called by background jobs for system monitoring
    ///
    /// **Use Cases:**
    /// - System-wide storage capacity monitoring
    /// - Infrastructure capacity planning
    /// - Billing/quota calculations
    ///
    /// **NOTE:** For tenant-scoped metrics, query the individual media tables
    /// with tenant_id filtering directly.
    #[tracing::instrument(skip(self), fields(db.table = "storage_metrics", db.operation = "calculate_and_insert"))]
    pub async fn calculate_storage_metrics(&self) -> Result<StorageMetrics, AppError> {
        // Get image stats aggregated across all tenants (system-level operation)
        let image_row = sqlx::query(
            "SELECT COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as total_bytes FROM media WHERE media_type = 'image'"
        )
        .fetch_one(&self.pool)
        .await?;

        let image_count: i64 = image_row.get("count");
        let image_bytes: i64 = image_row.get("total_bytes");

        // Get video stats
        let video_row = sqlx::query(
            "SELECT COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as total_bytes FROM media WHERE media_type = 'video'"
        )
        .fetch_one(&self.pool)
        .await?;

        let video_count: i64 = video_row.get("count");
        let video_bytes: i64 = video_row.get("total_bytes");

        // Get audio stats
        let audio_row = sqlx::query(
            "SELECT COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as total_bytes FROM media WHERE media_type = 'audio'"
        )
        .fetch_one(&self.pool)
        .await?;

        let audio_count: i64 = audio_row.get("count");
        let audio_bytes: i64 = audio_row.get("total_bytes");

        // Get document stats
        let document_row = sqlx::query(
            "SELECT COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as total_bytes FROM media WHERE media_type = 'document'"
        )
        .fetch_one(&self.pool)
        .await?;

        let document_count: i64 = document_row.get("count");
        let document_bytes: i64 = document_row.get("total_bytes");

        // Get breakdown by content type from images
        let image_type_rows = sqlx::query(
            "SELECT content_type, COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as bytes FROM media WHERE media_type = 'image' GROUP BY content_type"
        )
        .fetch_all(&self.pool)
        .await?;

        // Get breakdown by content type from videos
        let video_type_rows = sqlx::query(
            "SELECT content_type, COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as bytes FROM media WHERE media_type = 'video' GROUP BY content_type"
        )
        .fetch_all(&self.pool)
        .await?;

        // Get breakdown by content type from audios
        let audio_type_rows = sqlx::query(
            "SELECT content_type, COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as bytes FROM media WHERE media_type = 'audio' GROUP BY content_type"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut by_content_type: HashMap<String, ContentTypeStats> = HashMap::new();

        for row in image_type_rows {
            let content_type: String = row.get("content_type");
            let count: i64 = row.get("count");
            let bytes: i64 = row.get("bytes");
            by_content_type.insert(content_type, ContentTypeStats { count, bytes });
        }

        for row in video_type_rows {
            let content_type: String = row.get("content_type");
            let count: i64 = row.get("count");
            let bytes: i64 = row.get("bytes");
            by_content_type
                .entry(content_type)
                .and_modify(|stats| {
                    stats.count += count;
                    stats.bytes += bytes;
                })
                .or_insert(ContentTypeStats { count, bytes });
        }

        for row in audio_type_rows {
            let content_type: String = row.get("content_type");
            let count: i64 = row.get("count");
            let bytes: i64 = row.get("bytes");
            by_content_type
                .entry(content_type)
                .and_modify(|stats| {
                    stats.count += count;
                    stats.bytes += bytes;
                })
                .or_insert(ContentTypeStats { count, bytes });
        }

        // Get breakdown by content type from documents
        let document_type_rows = sqlx::query(
            "SELECT content_type, COUNT(*)::BIGINT as count, (COALESCE(SUM(file_size), 0))::BIGINT as bytes FROM media WHERE media_type = 'document' GROUP BY content_type"
        )
        .fetch_all(&self.pool)
        .await?;

        for row in document_type_rows {
            let content_type: String = row.get("content_type");
            let count: i64 = row.get("count");
            let bytes: i64 = row.get("bytes");
            by_content_type
                .entry(content_type)
                .and_modify(|stats| {
                    stats.count += count;
                    stats.bytes += bytes;
                })
                .or_insert(ContentTypeStats { count, bytes });
        }

        let by_content_type_json =
            serde_json::to_value(&by_content_type).map_err(|e| sqlx::Error::Decode(Box::new(e)))?;

        let total_files = image_count + video_count + audio_count + document_count;
        let total_storage_bytes = image_bytes + video_bytes + audio_bytes + document_bytes;

        // Insert into storage_metrics table (tenant_id NULL = system-wide snapshot)
        let metrics = sqlx::query_as::<Postgres, StorageMetrics>(
            r#"
            INSERT INTO storage_metrics (
                tenant_id, total_files, total_storage_bytes, image_count, image_bytes,
                video_count, video_bytes, audio_count, audio_bytes,
                document_count, document_bytes, by_content_type
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(Option::<Uuid>::None)
        .bind(total_files)
        .bind(total_storage_bytes)
        .bind(image_count)
        .bind(image_bytes)
        .bind(video_count)
        .bind(video_bytes)
        .bind(audio_count)
        .bind(audio_bytes)
        .bind(document_count)
        .bind(document_bytes)
        .bind(by_content_type_json)
        .fetch_one(&self.pool)
        .await?;

        Ok(metrics)
    }

    #[tracing::instrument(skip(self), fields(db.table = "storage_metrics", db.operation = "select"))]
    pub async fn get_latest_storage_metrics(&self) -> Result<Option<StorageMetrics>, AppError> {
        let metrics = sqlx::query_as::<Postgres, StorageMetrics>(
            "SELECT * FROM storage_metrics ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(metrics)
    }
}

/// Factory function to create the appropriate analytics repository based on configuration
pub async fn create_analytics_repository(
    config: &mindia_core::Config,
    postgres_pool: PgPool,
) -> Result<Box<dyn AnalyticsRepositoryTrait>, AppError> {
    let analytics_db_type = config.analytics_db_type();

    match analytics_db_type {
        Some("postgres") | None => {
            // Default to PostgreSQL
            tracing::info!("Initializing PostgreSQL analytics repository");
            let repo = PostgresAnalyticsRepository::new(postgres_pool);
            Ok(Box::new(repo))
        }
        Some(other) => Err(anyhow::anyhow!(
            "Invalid ANALYTICS_DB_TYPE: {}. Must be 'postgres' (or leave unset for default)",
            other
        )
        .into()),
    }
}
