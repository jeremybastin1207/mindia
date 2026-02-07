//! Configuration module
//!
//! This module provides configuration structures for the API and services,
//! including database, storage, authentication, and service-specific settings.

use std::env;

use crate::storage_types::StorageBackend;

// Common constants
const MAX_CONNECTIONS: u32 = 20;
const CONNECTION_TIMEOUT_SECS: u64 = 30;
const JWT_EXPIRY_HOURS: i64 = 24;
const HTTP_RATE_LIMIT_PER_MINUTE: u32 = 100;
const HTTP_TENANT_RATE_LIMIT_PER_MINUTE: u32 = 200;

/// Base configuration shared by both services
#[derive(Clone, Debug)]
pub struct BaseConfig {
    pub server_port: u16,
    pub cors_origins: Vec<String>,
    pub db_max_connections: u32,
    pub db_timeout_seconds: u64,
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub http_rate_limit_per_minute: u32,
    pub http_tenant_rate_limit_per_minute: Option<u32>,
    pub environment: String,
    // OpenTelemetry configuration
    pub otel_enabled: bool,
    pub otel_endpoint: String,
    pub otel_service_name: String,
    pub otel_service_version: String,
    pub otel_protocol: String,
    pub otel_sampler: String,
    pub otel_sample_ratio: f64,
    pub otel_metrics_interval_secs: u64,
}

/// Media processor configuration
#[derive(Clone, Debug)]
pub struct MediaProcessorConfig {
    pub base: BaseConfig,
    pub database_url: String,
    // Service API key (for service-to-service auth)
    pub service_api_key: Option<String>,
    // Storage configuration
    pub storage_backend: Option<StorageBackend>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
    pub s3_endpoint: Option<String>, // Custom endpoint for S3-compatible providers (MinIO, DigitalOcean Spaces, etc.)
    pub aws_region: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
    pub local_storage_path: Option<String>,
    pub local_storage_base_url: Option<String>,
    // Media processing configuration
    pub max_file_size_bytes: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
    pub remove_exif: bool,
    pub max_video_size_bytes: usize,
    pub video_allowed_extensions: Vec<String>,
    pub video_allowed_content_types: Vec<String>,
    pub ffmpeg_path: String,
    pub max_concurrent_transcodes: usize,
    pub hls_segment_duration: u64,
    pub hls_variants: Vec<String>,
    pub max_document_size_bytes: usize,
    pub document_allowed_extensions: Vec<String>,
    pub document_allowed_content_types: Vec<String>,
    pub max_audio_size_bytes: usize,
    pub audio_allowed_extensions: Vec<String>,
    pub audio_allowed_content_types: Vec<String>,
    // Analytics database configuration
    pub analytics_db_type: Option<String>,
    pub analytics_db_url: Option<String>,
    // ClamAV configuration
    pub clamav_enabled: bool,
    pub clamav_host: String,
    pub clamav_port: u16,
    pub clamav_fail_closed: bool,
    // Semantic search configuration
    // Uses Claude Vision (ANTHROPIC_API_KEY) + Voyage AI embeddings (VOYAGE_API_KEY)
    pub semantic_search_enabled: bool,
    pub semantic_search_provider: String,
    pub anthropic_api_key: Option<String>,
    pub voyage_api_key: Option<String>,
    pub anthropic_vision_model: String,
    pub voyage_embedding_model: String,
    // File storing behavior
    pub auto_store_enabled: bool,
    // URL upload allowlist (optional, defense in depth for SSRF prevention)
    // If set, only URLs from these domains are allowed for URL uploads
    // Example: "example.com,cdn.example.com,images.example.com"
    pub url_upload_allowlist: Option<Vec<String>>,
    // Task queue configuration
    pub task_queue_max_workers: usize,
    pub task_queue_poll_interval_ms: u64,
    pub task_queue_video_rate_limit: f64,
    pub task_queue_embedding_rate_limit: f64,
    pub task_queue_default_timeout_seconds: i32,
    pub task_queue_max_retries: i32,
    /// Interval in seconds between runs of the stale task reaper. 0 = disabled.
    pub task_queue_stale_task_reap_interval_secs: u64,
    /// Grace period in seconds added to task timeout before reaping stale running tasks.
    pub task_queue_stale_task_grace_period_secs: i64,
    /// Retention in days for finished tasks (completed/failed/cancelled). Old tasks are deleted during cleanup. 0 = disabled.
    pub task_retention_days: i32,
    // Capacity check configuration
    pub min_disk_free_gb: u64,
    pub max_memory_usage_percent: f64,
    pub max_cpu_usage_percent: f64,
    pub disk_check_behavior: String,
    pub memory_check_behavior: String,
    pub cpu_check_behavior: String,
    pub video_transcode_space_multiplier: f64,
    pub capacity_monitor_interval_secs: u64,
    pub capacity_monitor_enabled: bool,
    // Email / alert notifications
    pub email_alerts_enabled: bool,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_user: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_from: Option<String>,
    pub smtp_tls: bool,
    pub frontend_url: Option<String>,
}

/// Application configuration (media processor).
#[derive(Clone, Debug)]
pub struct Config(pub Box<MediaProcessorConfig>);

impl Config {
    fn as_media(&self) -> &MediaProcessorConfig {
        &self.0
    }

    /// Check if the application is running in production mode
    pub fn is_production(&self) -> bool {
        self.as_media()
            .base
            .environment
            .to_lowercase()
            .eq("production")
            || self.as_media().base.environment.to_lowercase().eq("prod")
    }

    pub fn from_env() -> Result<Self, anyhow::Error> {
        let config = MediaProcessorConfig::from_env()?;
        Ok(Config(Box::new(config)))
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        self.as_media().validate()
    }

    // Convenience getters for common fields
    pub fn server_port(&self) -> u16 {
        self.as_media().base.server_port
    }

    pub fn jwt_secret(&self) -> &str {
        &self.as_media().base.jwt_secret
    }

    pub fn jwt_expiry_hours(&self) -> i64 {
        self.as_media().base.jwt_expiry_hours
    }

    pub fn cors_origins(&self) -> &[String] {
        &self.as_media().base.cors_origins
    }

    pub fn http_rate_limit_per_minute(&self) -> u32 {
        self.as_media().base.http_rate_limit_per_minute
    }

    pub fn http_tenant_rate_limit_per_minute(&self) -> Option<u32> {
        self.as_media().base.http_tenant_rate_limit_per_minute
    }

    pub fn min_disk_free_gb(&self) -> u64 {
        self.as_media().min_disk_free_gb
    }

    pub fn max_memory_usage_percent(&self) -> f64 {
        self.as_media().max_memory_usage_percent
    }

    pub fn max_cpu_usage_percent(&self) -> f64 {
        self.as_media().max_cpu_usage_percent
    }

    pub fn disk_check_behavior(&self) -> String {
        self.as_media().disk_check_behavior.clone()
    }

    pub fn memory_check_behavior(&self) -> String {
        self.as_media().memory_check_behavior.clone()
    }

    pub fn cpu_check_behavior(&self) -> String {
        self.as_media().cpu_check_behavior.clone()
    }

    pub fn video_transcode_space_multiplier(&self) -> f64 {
        self.as_media().video_transcode_space_multiplier
    }

    pub fn capacity_monitor_enabled(&self) -> bool {
        self.as_media().capacity_monitor_enabled
    }

    pub fn auto_store_enabled(&self) -> bool {
        self.as_media().auto_store_enabled
    }

    pub fn ffmpeg_path(&self) -> &str {
        &self.as_media().ffmpeg_path
    }

    pub fn database_url(&self) -> &str {
        &self.as_media().database_url
    }

    pub fn allowed_extensions(&self) -> &[String] {
        &self.as_media().allowed_extensions
    }

    pub fn allowed_content_types(&self) -> &[String] {
        &self.as_media().allowed_content_types
    }

    pub fn max_file_size_bytes(&self) -> usize {
        self.as_media().max_file_size_bytes
    }

    pub fn max_video_size_bytes(&self) -> usize {
        self.as_media().max_video_size_bytes
    }

    pub fn max_audio_size_bytes(&self) -> usize {
        self.as_media().max_audio_size_bytes
    }

    pub fn max_document_size_bytes(&self) -> usize {
        self.as_media().max_document_size_bytes
    }

    pub fn video_allowed_extensions(&self) -> &[String] {
        &self.as_media().video_allowed_extensions
    }

    pub fn video_allowed_content_types(&self) -> &[String] {
        &self.as_media().video_allowed_content_types
    }

    pub fn audio_allowed_extensions(&self) -> &[String] {
        &self.as_media().audio_allowed_extensions
    }

    pub fn audio_allowed_content_types(&self) -> &[String] {
        &self.as_media().audio_allowed_content_types
    }

    pub fn document_allowed_extensions(&self) -> &[String] {
        &self.as_media().document_allowed_extensions
    }

    pub fn document_allowed_content_types(&self) -> &[String] {
        &self.as_media().document_allowed_content_types
    }

    pub fn remove_exif(&self) -> bool {
        self.as_media().remove_exif
    }

    pub fn max_concurrent_transcodes(&self) -> usize {
        self.as_media().max_concurrent_transcodes
    }

    pub fn hls_segment_duration(&self) -> u64 {
        self.as_media().hls_segment_duration
    }

    pub fn hls_variants(&self) -> &[String] {
        &self.as_media().hls_variants
    }

    pub fn s3_bucket(&self) -> Option<&str> {
        self.as_media().s3_bucket.as_deref()
    }

    pub fn s3_region(&self) -> Option<&str> {
        self.as_media().s3_region.as_deref()
    }

    pub fn s3_endpoint(&self) -> Option<&str> {
        self.as_media().s3_endpoint.as_deref()
    }

    pub fn aws_region(&self) -> Option<&str> {
        self.as_media().aws_region.as_deref()
    }

    pub fn clamav_enabled(&self) -> bool {
        self.as_media().clamav_enabled
    }

    pub fn clamav_host(&self) -> &str {
        &self.as_media().clamav_host
    }

    pub fn clamav_port(&self) -> u16 {
        self.as_media().clamav_port
    }

    pub fn clamav_fail_closed(&self) -> bool {
        self.as_media().clamav_fail_closed
    }

    pub fn semantic_search_enabled(&self) -> bool {
        self.as_media().semantic_search_enabled
    }

    pub fn semantic_search_provider(&self) -> &str {
        &self.as_media().semantic_search_provider
    }

    pub fn anthropic_api_key(&self) -> Option<&str> {
        self.as_media().anthropic_api_key.as_deref()
    }

    pub fn anthropic_vision_model(&self) -> &str {
        &self.as_media().anthropic_vision_model
    }

    pub fn voyage_embedding_model(&self) -> &str {
        &self.as_media().voyage_embedding_model
    }

    pub fn voyage_api_key(&self) -> Option<&str> {
        self.as_media().voyage_api_key.as_deref()
    }

    pub fn capacity_monitor_interval_secs(&self) -> u64 {
        self.as_media().capacity_monitor_interval_secs
    }

    pub fn task_queue_max_workers(&self) -> usize {
        self.as_media().task_queue_max_workers
    }

    pub fn task_queue_poll_interval_ms(&self) -> u64 {
        self.as_media().task_queue_poll_interval_ms
    }

    pub fn task_queue_video_rate_limit(&self) -> f64 {
        self.as_media().task_queue_video_rate_limit
    }

    pub fn task_queue_embedding_rate_limit(&self) -> f64 {
        self.as_media().task_queue_embedding_rate_limit
    }

    pub fn task_queue_default_timeout_seconds(&self) -> i32 {
        self.as_media().task_queue_default_timeout_seconds
    }

    pub fn task_queue_max_retries(&self) -> i32 {
        self.as_media().task_queue_max_retries
    }

    pub fn task_queue_stale_task_reap_interval_secs(&self) -> u64 {
        self.as_media().task_queue_stale_task_reap_interval_secs
    }

    pub fn task_queue_stale_task_grace_period_secs(&self) -> i64 {
        self.as_media().task_queue_stale_task_grace_period_secs
    }

    pub fn task_retention_days(&self) -> i32 {
        self.as_media().task_retention_days
    }

    pub fn environment(&self) -> &str {
        &self.as_media().base.environment
    }

    pub fn db_max_connections(&self) -> u32 {
        self.as_media().base.db_max_connections
    }

    pub fn db_timeout_seconds(&self) -> u64 {
        self.as_media().base.db_timeout_seconds
    }

    pub fn auth0_domain(&self) -> Option<&str> {
        None
    }

    pub fn auth0_client_id(&self) -> Option<&str> {
        None
    }

    pub fn auth0_client_secret(&self) -> Option<&str> {
        None
    }

    pub fn auth0_audience(&self) -> Option<&str> {
        None
    }

    pub fn webhook_timeout_seconds(&self) -> u64 {
        30
    }

    pub fn webhook_max_retries(&self) -> u32 {
        3
    }

    pub fn webhook_max_concurrent_deliveries(&self) -> usize {
        10
    }

    pub fn webhook_max_concurrent_retries(&self) -> usize {
        5
    }

    pub fn webhook_retry_poll_interval_seconds(&self) -> u64 {
        60
    }

    pub fn webhook_retry_batch_size(&self) -> usize {
        100
    }

    pub fn otel_enabled(&self) -> bool {
        self.as_media().base.otel_enabled
    }

    pub fn otel_endpoint(&self) -> Option<&str> {
        let ep = &self.as_media().base.otel_endpoint;
        if ep.is_empty() {
            None
        } else {
            Some(ep.as_str())
        }
    }

    pub fn otel_service_name(&self) -> &str {
        &self.as_media().base.otel_service_name
    }

    pub fn otel_service_version(&self) -> &str {
        &self.as_media().base.otel_service_version
    }

    pub fn otel_protocol(&self) -> &str {
        &self.as_media().base.otel_protocol
    }

    pub fn otel_sampler(&self) -> &str {
        &self.as_media().base.otel_sampler
    }

    pub fn otel_sample_ratio(&self) -> f64 {
        self.as_media().base.otel_sample_ratio
    }

    pub fn otel_metrics_interval_secs(&self) -> u64 {
        self.as_media().base.otel_metrics_interval_secs
    }

    pub fn email_alerts_enabled(&self) -> bool {
        self.as_media().email_alerts_enabled
    }

    pub fn smtp_host(&self) -> Option<&str> {
        self.as_media().smtp_host.as_deref()
    }

    pub fn smtp_port(&self) -> Option<u16> {
        self.as_media().smtp_port
    }

    pub fn smtp_user(&self) -> Option<&str> {
        self.as_media().smtp_user.as_deref()
    }

    pub fn smtp_password(&self) -> Option<&str> {
        self.as_media().smtp_password.as_deref()
    }

    pub fn smtp_from(&self) -> Option<&str> {
        self.as_media().smtp_from.as_deref()
    }

    pub fn smtp_tls(&self) -> bool {
        self.as_media().smtp_tls
    }

    pub fn frontend_url(&self) -> Option<&str> {
        self.as_media().frontend_url.as_deref()
    }

    pub fn url_upload_allowlist(&self) -> Option<&[String]> {
        self.as_media().url_upload_allowlist.as_deref()
    }

    pub fn analytics_db_type(&self) -> Option<&str> {
        self.as_media().analytics_db_type.as_deref()
    }

    pub fn storage_backend(&self) -> Option<crate::storage_types::StorageBackend> {
        self.as_media().storage_backend
    }

    pub fn local_storage_path(&self) -> Option<&str> {
        self.as_media().local_storage_path.as_deref()
    }

    pub fn local_storage_base_url(&self) -> Option<&str> {
        self.as_media().local_storage_base_url.as_deref()
    }
}

impl MediaProcessorConfig {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        dotenvy::dotenv().ok();

        const MAX_FILE_SIZE_MB: usize = 10;
        const MAX_VIDEO_SIZE_MB: usize = 500;
        const MAX_DOCUMENT_SIZE_MB: usize = 50;
        const MAX_AUDIO_SIZE_MB: usize = 100;
        const MAX_CONCURRENT_TRANSCODES: usize = 2;
        const HLS_SEGMENT_DURATION: u64 = 6;
        const TASK_QUEUE_MAX_WORKERS: usize = 4;
        const TASK_QUEUE_POLL_INTERVAL_MS: u64 = 1000;
        const TASK_QUEUE_VIDEO_RATE_LIMIT: f64 = 2.0;
        const TASK_QUEUE_EMBEDDING_RATE_LIMIT: f64 = 5.0;
        const TASK_QUEUE_DEFAULT_TIMEOUT_SECS: i32 = 3600;
        const TASK_QUEUE_MAX_RETRIES: i32 = 3;
        const STALE_TASK_REAP_INTERVAL_SECS: u64 = 60;
        const STALE_TASK_GRACE_PERIOD_SECS: i64 = 300;
        const TASK_RETENTION_DAYS: i32 = 30;
        const MIN_DISK_FREE_GB: u64 = 10;
        const MAX_MEMORY_USAGE_PERCENT: f64 = 85.0;
        const MAX_CPU_USAGE_PERCENT: f64 = 90.0;
        const VIDEO_TRANSCODE_SPACE_MULTIPLIER: f64 = 4.0;
        const CAPACITY_MONITOR_INTERVAL_SECS: u64 = 5;

        let environment = env::var("ENVIRONMENT")
            .or_else(|_| env::var("APP_ENV"))
            .unwrap_or_else(|_| "development".to_string());

        let cors_origins_str = env::var("CORS_ORIGINS").unwrap_or_else(|_| "*".to_string());
        let is_production =
            environment.to_lowercase() == "production" || environment.to_lowercase() == "prod";
        if is_production && cors_origins_str.trim() == "*" {
            return Err(anyhow::anyhow!(
                "CORS_ORIGINS cannot be '*' in production. Please specify explicit origins."
            ));
        }

        let cors_origins: Vec<String> = cors_origins_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        let max_file_size_mb = env::var("MAX_FILE_SIZE_MB")
            .unwrap_or_else(|_| MAX_FILE_SIZE_MB.to_string())
            .parse::<usize>()
            .unwrap_or(MAX_FILE_SIZE_MB);

        let allowed_extensions = env::var("ALLOWED_EXTENSIONS")
            .unwrap_or_else(|_| "jpg,jpeg,png,gif,webp".to_string())
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();

        let allowed_content_types = env::var("ALLOWED_CONTENT_TYPES")
            .unwrap_or_else(|_| "image/jpeg,image/png,image/gif,image/webp".to_string())
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .collect();

        let base = BaseConfig {
            server_port: env::var("PORT")
                .unwrap_or_else(|_| "4000".to_string())
                .parse()
                .map_err(|_| anyhow::anyhow!("PORT must be a valid number"))?,
            cors_origins,
            db_max_connections: env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| MAX_CONNECTIONS.to_string())
                .parse()
                .unwrap_or(MAX_CONNECTIONS),
            db_timeout_seconds: env::var("DB_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| CONNECTION_TIMEOUT_SECS.to_string())
                .parse()
                .unwrap_or(CONNECTION_TIMEOUT_SECS),
            jwt_secret: env::var("JWT_SECRET")
                .map_err(|_| anyhow::anyhow!("JWT_SECRET must be set for authentication"))?,
            jwt_expiry_hours: env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| JWT_EXPIRY_HOURS.to_string())
                .parse()
                .unwrap_or(JWT_EXPIRY_HOURS),
            http_rate_limit_per_minute: env::var("HTTP_RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| HTTP_RATE_LIMIT_PER_MINUTE.to_string())
                .parse()
                .unwrap_or(HTTP_RATE_LIMIT_PER_MINUTE),
            http_tenant_rate_limit_per_minute: env::var("HTTP_TENANT_RATE_LIMIT_PER_MINUTE")
                .ok()
                .and_then(|s| s.parse().ok())
                .or(Some(HTTP_TENANT_RATE_LIMIT_PER_MINUTE)),
            environment: environment.clone(),
            otel_enabled: env::var("OTEL_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(true),
            otel_endpoint: env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4317".to_string()),
            otel_service_name: env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "mindia-api".to_string()),
            otel_service_version: env::var("OTEL_SERVICE_VERSION")
                .unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string()),
            otel_protocol: env::var("OTEL_EXPORTER_OTLP_PROTOCOL")
                .unwrap_or_else(|_| "grpc".to_string()),
            otel_sampler: env::var("OTEL_SAMPLER")
                .unwrap_or_else(|_| "always_on".to_string())
                .to_lowercase(),
            otel_sample_ratio: env::var("OTEL_SAMPLE_RATIO")
                .unwrap_or_else(|_| "1.0".to_string())
                .parse()
                .unwrap_or(1.0),
            otel_metrics_interval_secs: env::var("OTEL_METRICS_INTERVAL_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .unwrap_or(30),
        };

        // Storage backend configuration
        let storage_backend =
            env::var("STORAGE_BACKEND")
                .ok()
                .and_then(|s| match s.to_lowercase().as_str() {
                    "s3" => Some(StorageBackend::S3),
                    "local" => Some(StorageBackend::Local),
                    _ => None,
                });

        let config = MediaProcessorConfig {
            base,
            database_url: env::var("MEDIA_PROCESSOR_DATABASE_URL")
                .or_else(|_| env::var("DATABASE_URL"))
                .map_err(|_| {
                    anyhow::anyhow!("MEDIA_PROCESSOR_DATABASE_URL or DATABASE_URL must be set")
                })?,
            service_api_key: env::var("SERVICE_API_KEY").ok(),
            storage_backend,
            s3_bucket: env::var("S3_BUCKET").ok(),
            s3_region: env::var("S3_REGION").ok(),
            s3_endpoint: env::var("S3_ENDPOINT").ok(),
            aws_region: env::var("AWS_REGION").ok(),
            aws_access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
            aws_secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
            local_storage_path: env::var("LOCAL_STORAGE_PATH").ok(),
            local_storage_base_url: env::var("LOCAL_STORAGE_BASE_URL").ok(),
            max_file_size_bytes: max_file_size_mb * 1024 * 1024,
            allowed_extensions,
            allowed_content_types,
            remove_exif: env::var("REMOVE_EXIF")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(true),
            max_video_size_bytes: env::var("MAX_VIDEO_SIZE_MB")
                .unwrap_or_else(|_| MAX_VIDEO_SIZE_MB.to_string())
                .parse::<usize>()
                .unwrap_or(MAX_VIDEO_SIZE_MB)
                * 1024
                * 1024,
            video_allowed_extensions: env::var("VIDEO_ALLOWED_EXTENSIONS")
                .unwrap_or_else(|_| "mp4,mov,avi,webm,mkv".to_string())
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            video_allowed_content_types: env::var("VIDEO_ALLOWED_CONTENT_TYPES")
                .unwrap_or_else(|_| {
                    "video/mp4,video/quicktime,video/x-msvideo,video/webm,video/x-matroska"
                        .to_string()
                })
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            ffmpeg_path: env::var("FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string()),
            max_concurrent_transcodes: env::var("MAX_CONCURRENT_TRANSCODES")
                .unwrap_or_else(|_| MAX_CONCURRENT_TRANSCODES.to_string())
                .parse()
                .unwrap_or(MAX_CONCURRENT_TRANSCODES),
            hls_segment_duration: env::var("HLS_SEGMENT_DURATION")
                .unwrap_or_else(|_| HLS_SEGMENT_DURATION.to_string())
                .parse()
                .unwrap_or(HLS_SEGMENT_DURATION),
            hls_variants: env::var("HLS_VARIANTS")
                .unwrap_or_else(|_| "360p,480p,720p,1080p".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            max_document_size_bytes: env::var("MAX_DOCUMENT_SIZE_MB")
                .unwrap_or_else(|_| MAX_DOCUMENT_SIZE_MB.to_string())
                .parse::<usize>()
                .unwrap_or(MAX_DOCUMENT_SIZE_MB)
                * 1024
                * 1024,
            document_allowed_extensions: env::var("DOCUMENT_ALLOWED_EXTENSIONS")
                .unwrap_or_else(|_| "pdf".to_string())
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            document_allowed_content_types: env::var("DOCUMENT_ALLOWED_CONTENT_TYPES")
                .unwrap_or_else(|_| "application/pdf".to_string())
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            max_audio_size_bytes: env::var("MAX_AUDIO_SIZE_MB")
                .unwrap_or_else(|_| MAX_AUDIO_SIZE_MB.to_string())
                .parse::<usize>()
                .unwrap_or(MAX_AUDIO_SIZE_MB)
                * 1024
                * 1024,
            audio_allowed_extensions: env::var("AUDIO_ALLOWED_EXTENSIONS")
                .unwrap_or_else(|_| "mp3,m4a,wav,flac,ogg".to_string())
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            audio_allowed_content_types: env::var("AUDIO_ALLOWED_CONTENT_TYPES")
                .unwrap_or_else(|_| {
                    "audio/mpeg,audio/mp4,audio/x-m4a,audio/wav,audio/flac,audio/ogg".to_string()
                })
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .collect(),
            analytics_db_type: env::var("ANALYTICS_DB_TYPE").ok().map(|s| s.to_lowercase()),
            analytics_db_url: env::var("ANALYTICS_DB_URL").ok(),
            clamav_enabled: env::var("CLAMAV_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(false),
            clamav_host: env::var("CLAMAV_HOST").unwrap_or_else(|_| "localhost".to_string()),
            clamav_port: env::var("CLAMAV_PORT")
                .unwrap_or_else(|_| "3310".to_string())
                .parse()
                .unwrap_or(3310),
            clamav_fail_closed: env::var("CLAMAV_FAIL_CLOSED")
                .unwrap_or_else(|_| is_production.to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(is_production),
            semantic_search_enabled: env::var("SEMANTIC_SEARCH_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(false),
            semantic_search_provider: env::var("SEMANTIC_SEARCH_PROVIDER")
                .unwrap_or_else(|_| "anthropic".to_string())
                .to_lowercase(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            voyage_api_key: env::var("VOYAGE_API_KEY").ok(),
            anthropic_vision_model: env::var("ANTHROPIC_VISION_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string()),
            voyage_embedding_model: env::var("VOYAGE_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "voyage-3-large".to_string()),
            auto_store_enabled: env::var("AUTO_STORE_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(true),
            url_upload_allowlist: env::var("URL_UPLOAD_ALLOWLIST").ok().map(|s| {
                s.split(',')
                    .map(|domain| domain.trim().to_lowercase())
                    .filter(|domain| !domain.is_empty())
                    .collect()
            }),
            task_queue_max_workers: env::var("TASK_QUEUE_MAX_WORKERS")
                .unwrap_or_else(|_| TASK_QUEUE_MAX_WORKERS.to_string())
                .parse()
                .unwrap_or(TASK_QUEUE_MAX_WORKERS),
            task_queue_poll_interval_ms: env::var("TASK_QUEUE_POLL_INTERVAL_MS")
                .unwrap_or_else(|_| TASK_QUEUE_POLL_INTERVAL_MS.to_string())
                .parse()
                .unwrap_or(TASK_QUEUE_POLL_INTERVAL_MS),
            task_queue_video_rate_limit: env::var("TASK_QUEUE_VIDEO_RATE_LIMIT")
                .unwrap_or_else(|_| TASK_QUEUE_VIDEO_RATE_LIMIT.to_string())
                .parse()
                .unwrap_or(TASK_QUEUE_VIDEO_RATE_LIMIT),
            task_queue_embedding_rate_limit: env::var("TASK_QUEUE_EMBEDDING_RATE_LIMIT")
                .unwrap_or_else(|_| TASK_QUEUE_EMBEDDING_RATE_LIMIT.to_string())
                .parse()
                .unwrap_or(TASK_QUEUE_EMBEDDING_RATE_LIMIT),
            task_queue_default_timeout_seconds: env::var("TASK_QUEUE_DEFAULT_TIMEOUT_SECONDS")
                .unwrap_or_else(|_| TASK_QUEUE_DEFAULT_TIMEOUT_SECS.to_string())
                .parse()
                .unwrap_or(TASK_QUEUE_DEFAULT_TIMEOUT_SECS),
            task_queue_max_retries: env::var("TASK_QUEUE_MAX_RETRIES")
                .unwrap_or_else(|_| TASK_QUEUE_MAX_RETRIES.to_string())
                .parse()
                .unwrap_or(TASK_QUEUE_MAX_RETRIES),
            task_queue_stale_task_reap_interval_secs: env::var("STALE_TASK_REAP_INTERVAL_SECS")
                .unwrap_or_else(|_| STALE_TASK_REAP_INTERVAL_SECS.to_string())
                .parse()
                .unwrap_or(STALE_TASK_REAP_INTERVAL_SECS),
            task_queue_stale_task_grace_period_secs: env::var("STALE_TASK_GRACE_PERIOD_SECS")
                .unwrap_or_else(|_| STALE_TASK_GRACE_PERIOD_SECS.to_string())
                .parse()
                .unwrap_or(STALE_TASK_GRACE_PERIOD_SECS),
            task_retention_days: env::var("TASK_RETENTION_DAYS")
                .unwrap_or_else(|_| TASK_RETENTION_DAYS.to_string())
                .parse()
                .unwrap_or(TASK_RETENTION_DAYS),
            min_disk_free_gb: env::var("MIN_DISK_FREE_GB")
                .unwrap_or_else(|_| MIN_DISK_FREE_GB.to_string())
                .parse()
                .unwrap_or(MIN_DISK_FREE_GB),
            max_memory_usage_percent: env::var("MAX_MEMORY_USAGE_PERCENT")
                .unwrap_or_else(|_| MAX_MEMORY_USAGE_PERCENT.to_string())
                .parse()
                .unwrap_or(MAX_MEMORY_USAGE_PERCENT),
            max_cpu_usage_percent: env::var("MAX_CPU_USAGE_PERCENT")
                .unwrap_or_else(|_| MAX_CPU_USAGE_PERCENT.to_string())
                .parse()
                .unwrap_or(MAX_CPU_USAGE_PERCENT),
            disk_check_behavior: env::var("DISK_CHECK_BEHAVIOR")
                .unwrap_or_else(|_| "fail".to_string())
                .to_lowercase(),
            memory_check_behavior: env::var("MEMORY_CHECK_BEHAVIOR")
                .unwrap_or_else(|_| "warn".to_string())
                .to_lowercase(),
            cpu_check_behavior: env::var("CPU_CHECK_BEHAVIOR")
                .unwrap_or_else(|_| "warn".to_string())
                .to_lowercase(),
            video_transcode_space_multiplier: env::var("VIDEO_TRANSCODE_SPACE_MULTIPLIER")
                .unwrap_or_else(|_| VIDEO_TRANSCODE_SPACE_MULTIPLIER.to_string())
                .parse()
                .unwrap_or(VIDEO_TRANSCODE_SPACE_MULTIPLIER),
            capacity_monitor_interval_secs: env::var("CAPACITY_MONITOR_INTERVAL_SECS")
                .unwrap_or_else(|_| CAPACITY_MONITOR_INTERVAL_SECS.to_string())
                .parse()
                .unwrap_or(CAPACITY_MONITOR_INTERVAL_SECS),
            capacity_monitor_enabled: env::var("CAPACITY_MONITOR_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(true),
            email_alerts_enabled: env::var("EMAIL_ALERTS_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(false),
            smtp_host: env::var("SMTP_HOST").ok().filter(|s| !s.is_empty()),
            smtp_port: env::var("SMTP_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .filter(|&p| p > 0),
            smtp_user: env::var("SMTP_USER").ok().filter(|s| !s.is_empty()),
            smtp_password: env::var("SMTP_PASSWORD").ok().filter(|s| !s.is_empty()),
            smtp_from: env::var("SMTP_FROM").ok().filter(|s| !s.is_empty()),
            smtp_tls: env::var("SMTP_TLS")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                .parse()
                .unwrap_or(true),
            frontend_url: env::var("FRONTEND_URL").ok().filter(|s| !s.is_empty()),
        };

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.base.jwt_secret.len() < 32 {
            return Err(anyhow::anyhow!(
                "JWT_SECRET must be at least 32 characters long"
            ));
        }

        if !self.database_url.starts_with("postgresql://") {
            return Err(anyhow::anyhow!(
                "MEDIA_PROCESSOR_DATABASE_URL must be a valid PostgreSQL connection string"
            ));
        }

        if self.email_alerts_enabled && (self.smtp_host.is_none() || self.smtp_from.is_none()) {
            return Err(anyhow::anyhow!(
                "EMAIL_ALERTS_ENABLED=true requires SMTP_HOST and SMTP_FROM to be set"
            ));
        }

        // Validate storage backend configuration
        let backend = self.storage_backend.unwrap_or(StorageBackend::S3);
        match backend {
            StorageBackend::S3 => {
                if self.s3_bucket.is_none() {
                    return Err(anyhow::anyhow!(
                        "S3_BUCKET must be set when using S3 storage backend"
                    ));
                }
                if self.s3_region.is_none() && self.aws_region.is_none() {
                    return Err(anyhow::anyhow!(
                        "S3_REGION or AWS_REGION must be set when using S3 storage backend"
                    ));
                }
            }
            StorageBackend::Local => {
                if self.local_storage_path.is_none() {
                    return Err(anyhow::anyhow!(
                        "LOCAL_STORAGE_PATH must be set when using local storage backend"
                    ));
                }
                if self.local_storage_base_url.is_none() {
                    return Err(anyhow::anyhow!(
                        "LOCAL_STORAGE_BASE_URL must be set when using local storage backend"
                    ));
                }
            }
            StorageBackend::Nfs => {
                // NFS storage backend validation can be added here if needed
            }
        }

        Ok(())
    }
}
