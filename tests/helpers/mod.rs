pub mod auth;
pub mod fixtures;
pub mod storage;
pub mod workflows;

/// Returns the versioned API path for the current API version.
/// Usage: `api_path("/images")` -> `/api/v0/images` (when API_VERSION=v0).
pub fn api_path(path: &str) -> String {
    let prefix = mindia::constants::API_PREFIX;
    format!("{}{}", prefix, path)
}

use axum::Router;
use axum_test::TestServer;
use mindia::config::Config;
use mindia::db::{
    ApiKeyRepository, EmbeddingRepository, FileGroupRepository, FolderRepository, MediaRepository,
    PostgresAnalyticsRepository, StorageMetricsRepository, TaskRepository, TenantRepository,
    WebhookEventRepository, WebhookRepository, WebhookRetryRepository,
};
use mindia::handlers;
use mindia::services::storage::{LocalStorage, Storage, StorageBackend};
use mindia::services::{
    AnalyticsService, CleanupService, EmbeddingJobQueue, RateLimiter, S3Service,
    TaskQueue, TaskQueueConfig, VideoJobQueue, WebhookRetryService, WebhookService,
};
use mindia::state::{
    AppState, AudioConfig, DatabaseConfig, DocumentConfig, ImageConfig, MediaConfig, S3Config,
    SecurityConfig, VideoConfig,
};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use testcontainers::clients::Cli;
use testcontainers::{Container, RunnableImage};
use testcontainers_modules::postgres::Postgres;

/// Test application state
pub struct TestApp {
    pub server: TestServer,
    pub pool: sqlx::PgPool,
    pub _container: Container<'static, Postgres>,
    pub _temp_dir: TempDir,
}

impl TestApp {
    /// Get the HTTP test client
    pub fn client(&self) -> &TestServer {
        &self.server
    }

    /// Get the database pool
    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

/// Setup a test application with isolated database
pub async fn setup_test_app() -> TestApp {
    // Initialize testcontainers client
    let docker = Cli::default();

    // Start PostgreSQL container
    let postgres_image = Postgres::default();
    let container = docker.run(postgres_image);

    // Get connection details
    let connection_string = format!(
        "postgresql://postgres:postgres@localhost:{}/postgres",
        container.get_host_port_ipv4(5432).unwrap()
    );

    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&connection_string)
        .await
        .expect("Failed to connect to test database");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Create temporary directory for local storage
    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let storage_path = temp_dir.path().to_path_buf();
    let storage = Arc::new(
        LocalStorage::new(storage_path, "http://localhost:3000/media".to_string())
            .await
            .expect("Failed to create local storage"),
    );

    // Create test configuration
    let config = create_test_config();

    // Initialize repositories
    let analytics_repo = PostgresAnalyticsRepository::new(pool.clone());
    let storage_repo = StorageMetricsRepository::new(pool.clone());
    let analytics_service = AnalyticsService::new(Box::new(analytics_repo), storage_repo);

    let embedding_db = EmbeddingRepository::new(pool.clone());
    let task_db = TaskRepository::new(pool.clone());

    let webhook_db = WebhookRepository::new(pool.clone());
    let webhook_event_db = WebhookEventRepository::new(pool.clone());
    let webhook_retry_db = WebhookRetryRepository::new(pool.clone());

    let tenant_db = TenantRepository::new(pool.clone());
    let api_key_db = ApiKeyRepository::new(pool.clone());

    // Initialize folder and file group repositories
    let folder_db = FolderRepository::new(pool.clone());
    let file_group_db = FileGroupRepository::new(pool.clone());

    // Initialize media repository with storage
    let media_db = MediaRepository::new(pool.clone(), storage.clone());

    // Initialize S3Service only if using S3 backend (optional for local storage)
    // For tests, we use local storage, so S3Service is not required
    let s3_config = if config.storage_backend == Some(StorageBackend::S3) {
        let s3_service = S3Service::new(None, "us-east-1".to_string())
            .await
            .expect("Failed to create S3 service");
        Some(S3Config {
            service: s3_service.clone(),
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
        })
    } else {
        None
    };

    let security_config = SecurityConfig {
        clamav: None, // Disabled for tests
        clamav_enabled: false,
        cors_origins: vec!["*".to_string()],
    };

    let database_config = DatabaseConfig {
        max_connections: 5,
        timeout_seconds: 30,
    };

    let image_config = ImageConfig {
        repository: media_db.clone(),
        max_file_size: config.max_file_size_bytes,
        allowed_extensions: config.allowed_extensions.clone(),
        allowed_content_types: config.allowed_content_types.clone(),
        remove_exif: config.remove_exif,
    };

    let document_config = DocumentConfig {
        repository: media_db.clone(),
        max_file_size: config.max_document_size_bytes,
        allowed_extensions: config.document_allowed_extensions.clone(),
        allowed_content_types: config.document_allowed_content_types.clone(),
    };

    let audio_config = AudioConfig {
        repository: audio_db.clone(),
        max_file_size: config.max_audio_size_bytes,
        allowed_extensions: config.audio_allowed_extensions.clone(),
        allowed_content_types: config.audio_allowed_content_types.clone(),
    };

    let media_config = MediaConfig {
        repository: media_db.clone(),
        file_group_repository: file_group_db.clone(),
        storage: storage.clone(),
        image_max_file_size: config.max_file_size_bytes,
        image_allowed_extensions: config.allowed_extensions.clone(),
        image_allowed_content_types: config.allowed_content_types.clone(),
        remove_exif: config.remove_exif,
        video_max_file_size: config.max_video_size_bytes,
        video_allowed_extensions: config.video_allowed_extensions.clone(),
        video_allowed_content_types: config.video_allowed_content_types.clone(),
        ffmpeg_path: config.ffmpeg_path.clone(),
        hls_segment_duration: config.hls_segment_duration,
        hls_variants: config.hls_variants.clone(),
        audio_max_file_size: config.max_audio_size_bytes,
        audio_allowed_extensions: config.audio_allowed_extensions.clone(),
        audio_allowed_content_types: config.audio_allowed_content_types.clone(),
        document_max_file_size: config.max_document_size_bytes,
        document_allowed_extensions: config.document_allowed_extensions.clone(),
        document_allowed_content_types: config.document_allowed_content_types.clone(),
    };

    let video_db = media_db.clone();
    let video_config = VideoConfig {
        repository: media_db.clone(),
        job_queue: VideoJobQueue::placeholder(),
        max_file_size: config.max_video_size_bytes,
        allowed_extensions: config.video_allowed_extensions.clone(),
        allowed_content_types: config.video_allowed_content_types.clone(),
        ffmpeg_path: config.ffmpeg_path.clone(),
        hls_segment_duration: config.hls_segment_duration,
        hls_variants: config.hls_variants.clone(),
    };

    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        7,
    );

    let webhook_service_config = mindia::services::WebhookServiceConfig {
        timeout_seconds: config.webhook_timeout_seconds,
        max_retries: config.webhook_max_retries,
        max_concurrent_deliveries: config.webhook_max_concurrent_deliveries,
    };

    let webhook_service = WebhookService::new(
        webhook_db.clone(),
        webhook_event_db.clone(),
        webhook_retry_db.clone(),
        webhook_service_config,
    )
    .expect("Failed to create webhook service for tests");

    let webhook_retry_config = mindia::services::WebhookRetryServiceConfig {
        poll_interval_seconds: config.webhook_retry_poll_interval_seconds,
        batch_size: config.webhook_retry_batch_size,
        max_concurrent_retries: config.webhook_max_concurrent_retries,
    };

    let webhook_retry_service = WebhookRetryService::new(
        webhook_retry_db.clone(),
        Arc::new(webhook_service.clone()),
        webhook_retry_config,
    );

    let rate_limiter = RateLimiter::new(
        config.task_queue_video_rate_limit,
        config.task_queue_embedding_rate_limit,
    );

    let task_queue_config = TaskQueueConfig {
        max_workers: config.task_queue_max_workers,
        poll_interval_ms: config.task_queue_poll_interval_ms,
        default_timeout_seconds: config.task_queue_default_timeout_seconds,
        max_retries: config.task_queue_max_retries,
    };

    // Create content moderation handler if feature is enabled (uses plugin system)
    #[cfg(feature = "content-moderation")]
    let content_moderation_handler = {
        #[cfg(feature = "plugin")]
        {
            // Create an empty plugin registry for tests (plugins can be registered if needed)
            use mindia::plugins::PluginRegistry;
            let plugin_registry = Arc::new(PluginRegistry::new());
            mindia::ContentModerationTaskHandler::new(plugin_registry)
        }
        #[cfg(not(feature = "plugin"))]
        {
            panic!("Content moderation requires plugin feature for tests");
        }
    };

    // Create state using Arc::new_cyclic to handle circular reference
    let state = Arc::new_cyclic(|state_ref| {
        let task_queue = TaskQueue::new(
            task_db.clone(),
            rate_limiter.clone(),
            task_queue_config.clone(),
            state_ref.clone(),
            Some(pool.clone()),
        );

        AppState {
            db_pool: pool.clone(),
            media: media_config.clone(),
            image: image_config.clone(),
            video: video_config.clone(),
            document: document_config.clone(),
            audio: audio_config.clone(),
            s3: s3_config.clone(), // S3Config is optional (None for local storage)
            security: security_config.clone(),
            database: database_config.clone(),
            analytics: analytics_service.clone(),
            is_production: false,
            semantic_search: None,
            embedding_repository: embedding_db.clone(),
            metadata_search_repository: mindia::db::MetadataSearchRepository::new(pool.clone()),
            embedding_queue: None,
            cleanup_service: Some(cleanup_service.clone()),
            config: config.clone(),
            task_queue,
            task_repository: task_db.clone(),
            video_db: video_db.clone(),
            webhook_repository: webhook_db.clone(),
            webhook_event_repository: webhook_event_db.clone(),
            webhook_retry_repository: webhook_retry_db.clone(),
            webhook_service: webhook_service.clone(),
            webhook_retry_service: webhook_retry_service.clone(),
            folder_repository: folder_db.clone(),
            api_key_repository: api_key_db.clone(),
            tenant_repository: tenant_db.clone(),
            #[cfg(feature = "plugin")]
            plugin_registry: Arc::new(mindia::plugins::PluginRegistry::new()),
            #[cfg(feature = "plugin")]
            plugin_service: mindia::plugins::PluginService::new(
                Arc::new(mindia::plugins::PluginRegistry::new()),
                mindia::db::PluginConfigRepository::new(pool.clone()),
                mindia::db::PluginExecutionRepository::new(pool.clone()),
                task_queue.clone(),
            ),
            #[cfg(feature = "plugin")]
            plugin_config_repository: mindia::db::PluginConfigRepository::new(pool.clone()),
            #[cfg(feature = "plugin")]
            plugin_execution_repository: mindia::db::PluginExecutionRepository::new(pool.clone()),
            #[cfg(feature = "plugin")]
            plugin_task_handler: mindia::PluginTaskHandler::new(
                Arc::new(mindia::plugins::PluginRegistry::new()),
                mindia::db::PluginConfigRepository::new(pool.clone()),
                mindia::db::PluginExecutionRepository::new(pool.clone()),
            ),
            #[cfg(feature = "content-moderation")]
            content_moderation_handler,
            capacity_checker: Arc::new(mindia::services::CapacityChecker::new(config.clone())),
        }
    });

    let auth_middleware_state = mindia::auth::middleware::AuthState {
        master_api_key: "test-master-api-key-at-least-32-characters-long".to_string(),
        api_key_repository: api_key_db.clone(),
        tenant_repository: tenant_db.clone(),
    };

    // Create router
    let public_routes = Router::new().route(
        "/health",
        axum::routing::get(|| async { (axum::http::StatusCode::OK, "OK") }),
    );

    let api_key_routes = Router::new()
        .route(
            "/api/v0/api-keys",
            axum::routing::post(handlers::api_keys::create_api_key),
        )
        .route(
            "/api/v0/api-keys",
            axum::routing::get(handlers::api_keys::list_api_keys),
        )
        .route(
            "/api/v0/api-keys/:id",
            axum::routing::get(handlers::api_keys::get_api_key),
        )
        .route(
            "/api/v0/api-keys/:id",
            axum::routing::delete(handlers::api_keys::revoke_api_key),
        )
        .with_state(state.clone());

    let protected_routes = Router::new()
        .route(
            "/api/v0/config",
            axum::routing::get(handlers::config::get_config),
        )
        .route(
            "/api/v0/images",
            axum::routing::post(handlers::image_upload::upload_image),
        )
        .route(
            "/api/v0/images",
            axum::routing::get(handlers::image_get::list_images),
        )
        .route(
            "/api/v0/images/:id",
            axum::routing::get(handlers::image_get::get_image),
        )
        .route(
            "/api/v0/images/:id/file",
            axum::routing::get(handlers::image_download::download_image),
        )
        .route(
            "/api/v0/images/:id/*operations",
            axum::routing::get(handlers::transform::transform_image),
        )
        .route(
            "/api/v0/images/:id",
            axum::routing::delete(handlers::image_delete::delete_image),
        )
        .route(
            "/api/v0/videos",
            axum::routing::post(handlers::video_upload::upload_video),
        )
        .route(
            "/api/v0/videos",
            axum::routing::get(handlers::video_get::list_videos),
        )
        .route(
            "/api/v0/videos/:id",
            axum::routing::get(handlers::video_get::get_video),
        )
        .route(
            "/api/v0/videos/:id",
            axum::routing::delete(handlers::video_delete::delete_video),
        )
        .route(
            "/api/v0/documents",
            axum::routing::post(handlers::document_upload::upload_document),
        )
        .route(
            "/api/v0/documents",
            axum::routing::get(handlers::document_get::list_documents),
        )
        .route(
            "/api/v0/documents/:id",
            axum::routing::get(handlers::document_get::get_document),
        )
        .route(
            "/api/v0/documents/:id/file",
            axum::routing::get(handlers::document_download::download_document),
        )
        .route(
            "/api/v0/documents/:id",
            axum::routing::delete(handlers::document_delete::delete_document),
        )
        .route(
            "/api/v0/audios",
            axum::routing::post(handlers::audio_upload::upload_audio),
        )
        .route(
            "/api/v0/audios",
            axum::routing::get(handlers::audio_get::list_audios),
        )
        .route(
            "/api/v0/audios/:id",
            axum::routing::get(handlers::audio_get::get_audio),
        )
        .route(
            "/api/v0/audios/:id/file",
            axum::routing::get(handlers::audio_download::download_audio),
        )
        .route(
            "/api/v0/audios/:id",
            axum::routing::delete(handlers::audio_delete::delete_audio),
        )
        .route(
            "/api/v0/analytics/traffic",
            axum::routing::get(handlers::analytics::get_traffic_summary),
        )
        .route(
            "/api/v0/analytics/urls",
            axum::routing::get(handlers::analytics::get_url_statistics),
        )
        .route(
            "/api/v0/analytics/storage",
            axum::routing::get(handlers::analytics::get_storage_summary),
        )
        .route(
            "/api/v0/analytics/storage/refresh",
            axum::routing::post(handlers::analytics::refresh_storage_metrics),
        )
        .route(
            "/api/v0/search",
            axum::routing::get(handlers::search::search_files),
        )
        .route(
            "/api/v0/tasks",
            axum::routing::get(handlers::tasks::list_tasks),
        )
        .route(
            "/api/v0/tasks/:id",
            axum::routing::get(handlers::tasks::get_task),
        )
        .route(
            "/api/v0/tasks/:id/cancel",
            axum::routing::post(handlers::tasks::cancel_task),
        )
        .route(
            "/api/v0/tasks/:id/retry",
            axum::routing::post(handlers::tasks::retry_task),
        )
        .route(
            "/api/v0/tasks/stats",
            axum::routing::get(handlers::tasks::get_task_stats),
        )
        .route(
            "/api/v0/webhooks",
            axum::routing::post(handlers::webhooks::create_webhook),
        )
        .route(
            "/api/v0/webhooks",
            axum::routing::get(handlers::webhooks::list_webhooks),
        )
        .route(
            "/api/v0/webhooks/:id",
            axum::routing::get(handlers::webhooks::get_webhook),
        )
        .route(
            "/api/v0/webhooks/:id",
            axum::routing::put(handlers::webhooks::update_webhook),
        )
        .route(
            "/api/v0/webhooks/:id",
            axum::routing::delete(handlers::webhooks::delete_webhook),
        )
        .route(
            "/api/v0/webhooks/:id/test",
            axum::routing::post(handlers::webhooks::test_webhook),
        )
        .route(
            "/api/v0/webhooks/:id/events",
            axum::routing::get(handlers::webhooks::list_webhook_events),
        )
        .with_state(state.clone());

    // Setup rate limiter for tests (lower limit to enable testing)
    use mindia::middleware::rate_limit::{rate_limit_middleware, HttpRateLimiter};
    let http_rate_limiter = Arc::new(HttpRateLimiter::with_tenant_limit(
        config.http_rate_limit_per_minute,
        config.http_rate_limit_per_minute, // Same limit for tenant in tests
    ));

    let all_protected_routes = Router::new()
        .merge(protected_routes)
        .merge(api_key_routes)
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(auth_middleware_state),
            mindia::auth::middleware::auth_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            http_rate_limiter.clone(),
            rate_limit_middleware,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(all_protected_routes);

    // Create test server
    let server = TestServer::new(app.into_make_service()).expect("Failed to create test server");

    TestApp {
        server,
        pool,
        _container: container,
        _temp_dir: temp_dir,
    }
}

/// Create test configuration
fn create_test_config() -> Config {
    Config {
        database_url: "postgresql://test".to_string(), // Not used, we override with container
        storage_backend: Some(StorageBackend::Local),
        s3_bucket: None,
        s3_region: None,
        aws_region: Some("us-east-1".to_string()),
        aws_access_key_id: None,
        aws_secret_access_key: None,
        local_storage_path: Some("/tmp/mindia-test".to_string()),
        local_storage_base_url: Some("http://localhost:3000/media".to_string()),
        server_port: 3000,
        max_file_size_bytes: 10 * 1024 * 1024, // 10MB
        allowed_extensions: vec![
            "jpg".to_string(),
            "jpeg".to_string(),
            "png".to_string(),
            "gif".to_string(),
            "webp".to_string(),
        ],
        allowed_content_types: vec![
            "image/jpeg".to_string(),
            "image/png".to_string(),
            "image/gif".to_string(),
            "image/webp".to_string(),
        ],
        cors_origins: vec!["*".to_string()],
        db_max_connections: 5,
        db_timeout_seconds: 30,
        clamav_enabled: false,
        clamav_host: "localhost".to_string(),
        clamav_port: 3310,
        clamav_fail_closed: false,
        remove_exif: false,
        max_video_size_bytes: 100 * 1024 * 1024, // 100MB
        video_allowed_extensions: vec!["mp4".to_string(), "mov".to_string()],
        video_allowed_content_types: vec!["video/mp4".to_string(), "video/quicktime".to_string()],
        ffmpeg_path: "ffmpeg".to_string(),
        max_concurrent_transcodes: 1,
        hls_segment_duration: 6,
        hls_variants: vec!["360p".to_string(), "720p".to_string()],
        max_document_size_bytes: 50 * 1024 * 1024, // 50MB
        document_allowed_extensions: vec!["pdf".to_string()],
        document_allowed_content_types: vec!["application/pdf".to_string()],
        max_audio_size_bytes: 20 * 1024 * 1024, // 20MB
        audio_allowed_extensions: vec!["mp3".to_string(), "wav".to_string()],
        audio_allowed_content_types: vec!["audio/mpeg".to_string(), "audio/wav".to_string()],
        otel_enabled: false,
        otel_endpoint: "".to_string(),
        otel_service_name: "mindia-test".to_string(),
        otel_service_version: "0.1.0".to_string(),
        otel_protocol: "grpc".to_string(),
        environment: "test".to_string(),
        semantic_search_enabled: false,
        semantic_search_provider: "anthropic".to_string(),
        anthropic_api_key: None,
        anthropic_vision_model: "claude-sonnet-4-20250514".to_string(),
        voyage_api_key: None,
        voyage_embedding_model: "voyage-3-large".to_string(),
        auto_store_enabled: true,
        task_queue_max_workers: 2,
        task_queue_poll_interval_ms: 1000,
        task_queue_video_rate_limit: 1.0,
        task_queue_embedding_rate_limit: 5.0,
        task_queue_default_timeout_seconds: 3600,
        task_queue_max_retries: 3,
        jwt_secret: "test-secret-key-min-32-characters-long-for-testing".to_string(),
        jwt_expiry_hours: 24,
        webhook_timeout_seconds: 30,
        webhook_max_retries: 3,
        webhook_retry_poll_interval_seconds: 30,
        webhook_retry_batch_size: 10,
        webhook_max_concurrent_retries: 2,
        http_rate_limit_per_minute: 10, // Lower limit for testing rate limiting
        http_tenant_rate_limit_per_minute: Some(20), // Higher limit for tenants in tests
        min_disk_free_gb: 1,
        max_memory_usage_percent: 90.0,
        max_cpu_usage_percent: 95.0,
        disk_check_behavior: "warn".to_string(),
        memory_check_behavior: "warn".to_string(),
        cpu_check_behavior: "warn".to_string(),
        video_transcode_space_multiplier: 4.0,
        capacity_monitor_interval_secs: 5,
        capacity_monitor_enabled: false,
    }
}
