//! Test helpers: build AppState and router for integration tests.
//!
//! Run from workspace root: `cargo test -p mindia-api --test images_test` or
//! `cargo test -p mindia-api`. Migrations path: from mindia-api crate root, `../../migrations`.

pub mod auth;
pub mod fixtures;
pub mod storage;
pub mod workflows;

use axum::Router;
use axum_test::TestServer;
use mindia_api::constants;
use mindia_api::setup::routes;
use mindia_api::state::{
    AppState, DatabaseConfig, DbState, MediaConfig, SecurityConfig, TaskState, WebhookState,
};
use mindia_core::{BaseConfig, Config, MediaProcessorConfig, StorageBackend};
use mindia_db::{
    create_analytics_repository, ApiKeyRepository, EmbeddingRepository, FileGroupRepository,
    FolderRepository, MediaRepository, MetadataSearchRepository, NamedTransformationRepository,
    StorageMetricsRepository, TaskRepository, TenantRepository, WebhookEventRepository,
    WebhookRepository, WebhookRetryRepository,
};
use mindia_infra::{
    AnalyticsService, CapacityChecker, CleanupService, RateLimiter, WebhookRetryService,
    WebhookRetryServiceConfig, WebhookService, WebhookServiceConfig,
};
use mindia_services::{LocalStorage, Storage};
use mindia_worker::{TaskQueue, TaskQueueConfig};
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use testcontainers::clients::Cli;
use testcontainers::{Container, RunnableImage};
use testcontainers_modules::postgres::Postgres;

/// API path prefix for tests (e.g. `/api/v0`).
pub fn api_path(path: &str) -> String {
    format!("{}{}", constants::API_PREFIX, path)
}

/// Test application: server, pool, and owned resources.
pub struct TestApp {
    pub server: TestServer,
    pub pool: sqlx::PgPool,
    pub _container: Container<'static, Postgres>,
    pub _temp_dir: TempDir,
}

impl TestApp {
    pub fn client(&self) -> &TestServer {
        &self.server
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }
}

/// Setup test app with isolated DB and local storage.
pub async fn setup_test_app() -> TestApp {
    std::env::set_var("MASTER_API_KEY", auth::TEST_MASTER_API_KEY);
    // 32-byte key base64 for plugin encryption when feature is enabled
    std::env::set_var(
        "ENCRYPTION_KEY",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
    );

    let docker = Cli::default();
    let postgres_image = Postgres::default();
    let container = docker.run(postgres_image);

    let connection_string = format!(
        "postgresql://postgres:postgres@localhost:{}/postgres",
        container.get_host_port_ipv4(5432).unwrap()
    );

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&connection_string)
        .await
        .expect("Failed to connect to test database");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let temp_dir = tempfile::tempdir().expect("Failed to create temp directory");
    let storage_path = temp_dir.path().to_path_buf();
    let storage: Arc<dyn Storage> = Arc::new(
        LocalStorage::new(storage_path, "http://localhost:3000/media".to_string())
            .await
            .expect("Failed to create local storage"),
    );

    let config = create_test_config(&connection_string);

    let analytics_repo = create_analytics_repository(&config, pool.clone())
        .await
        .expect("Failed to create analytics repo");
    let storage_repo = StorageMetricsRepository::new(pool.clone());
    let analytics_service = AnalyticsService::new(analytics_repo, storage_repo);

    let embedding_db = EmbeddingRepository::new(pool.clone());
    let metadata_search_db = MetadataSearchRepository::new(pool.clone());
    let task_db = TaskRepository::new(pool.clone());
    let webhook_db = WebhookRepository::new(pool.clone());
    let webhook_event_db = WebhookEventRepository::new(pool.clone());
    let webhook_retry_db = WebhookRetryRepository::new(pool.clone());
    let tenant_db = TenantRepository::new(pool.clone());
    let api_key_db = ApiKeyRepository::new(pool.clone());
    let folder_db = FolderRepository::new(pool.clone());
    let file_group_db = FileGroupRepository::new(pool.clone(), storage.clone());
    let named_transformation_db = NamedTransformationRepository::new(pool.clone());
    let media_db = MediaRepository::new(pool.clone(), storage.clone());

    let cleanup_service = CleanupService::new(
        Arc::new(media_db.clone()),
        storage.clone(),
        Some(Arc::new(task_db.clone())),
        7,
    );

    let webhook_service_config = WebhookServiceConfig {
        timeout_seconds: config.webhook_timeout_seconds(),
        max_retries: config.webhook_max_retries() as i32,
        max_concurrent_deliveries: config.webhook_max_concurrent_deliveries(),
    };
    let webhook_service = WebhookService::new(
        webhook_db.clone(),
        webhook_event_db.clone(),
        webhook_retry_db.clone(),
        webhook_service_config,
    )
    .expect("Failed to create webhook service");

    let webhook_retry_config = WebhookRetryServiceConfig {
        poll_interval_seconds: config.webhook_retry_poll_interval_seconds(),
        batch_size: config.webhook_retry_batch_size() as i64,
        max_concurrent_retries: config.webhook_max_concurrent_retries(),
    };
    let webhook_retry_service = WebhookRetryService::new(
        webhook_retry_db.clone(),
        Arc::new(webhook_service.clone()),
        webhook_retry_config,
    );

    let database_config = DatabaseConfig {
        max_connections: 5,
        timeout_seconds: 30,
    };

    let media_config = MediaConfig {
        repository: media_db.clone(),
        file_group_repository: file_group_db,
        storage: storage.clone(),
        image_max_file_size: config.max_file_size_bytes(),
        image_allowed_extensions: config.allowed_extensions().to_vec(),
        image_allowed_content_types: config.allowed_content_types().to_vec(),
        remove_exif: config.remove_exif(),
        video_max_file_size: config.max_video_size_bytes(),
        video_allowed_extensions: config.video_allowed_extensions().to_vec(),
        video_allowed_content_types: config.video_allowed_content_types().to_vec(),
        ffmpeg_path: config.ffmpeg_path().to_string(),
        hls_segment_duration: config.hls_segment_duration(),
        hls_variants: config.hls_variants().to_vec(),
        audio_max_file_size: config.max_audio_size_bytes(),
        audio_allowed_extensions: config.audio_allowed_extensions().to_vec(),
        audio_allowed_content_types: config.audio_allowed_content_types().to_vec(),
        document_max_file_size: config.max_document_size_bytes(),
        document_allowed_extensions: config.document_allowed_extensions().to_vec(),
        document_allowed_content_types: config.document_allowed_content_types().to_vec(),
    };

    #[cfg(feature = "clamav")]
    let security_config = SecurityConfig {
        clamav: None,
        clamav_enabled: false,
        cors_origins: vec!["*".to_string()],
    };
    #[cfg(not(feature = "clamav"))]
    let security_config = SecurityConfig {
        clamav_enabled: false,
        cors_origins: vec!["*".to_string()],
    };

    let rate_limiter = RateLimiter::new(
        config.task_queue_video_rate_limit(),
        config.task_queue_embedding_rate_limit(),
    );
    let task_queue_config = TaskQueueConfig {
        max_workers: config.task_queue_max_workers(),
        poll_interval_ms: config.task_queue_poll_interval_ms(),
        default_timeout_seconds: config.task_queue_default_timeout_seconds(),
        max_retries: config.task_queue_max_retries(),
        stale_task_reap_interval_secs: config.task_queue_stale_task_reap_interval_secs(),
        stale_task_grace_period_secs: config.task_queue_stale_task_grace_period_secs(),
    };
    let task_queue = TaskQueue::new_no_worker(task_db.clone(), rate_limiter, task_queue_config);

    #[cfg(feature = "video")]
    let video_job_queue = mindia_api::VideoJobQueue::dummy();

    #[cfg(all(feature = "content-moderation", feature = "video"))]
    let tasks = {
        #[cfg(feature = "plugin")]
        let content_moderation_handler =
            mindia_api::task_handlers::ContentModerationTaskHandler::new(Arc::new(
                mindia_api::plugins::PluginRegistry::new(),
            ));
        #[cfg(not(feature = "plugin"))]
        let content_moderation_handler = panic!("content-moderation requires plugin for tests");
        TaskState {
            task_queue: task_queue.clone(),
            task_repository: task_db.clone(),
            content_moderation_handler,
            video_job_queue,
        }
    };
    #[cfg(all(feature = "content-moderation", not(feature = "video")))]
    let tasks = {
        #[cfg(feature = "plugin")]
        let content_moderation_handler =
            mindia_api::task_handlers::ContentModerationTaskHandler::new(Arc::new(
                mindia_api::plugins::PluginRegistry::new(),
            ));
        #[cfg(not(feature = "plugin"))]
        let content_moderation_handler = panic!("content-moderation requires plugin for tests");
        TaskState {
            task_queue: task_queue.clone(),
            task_repository: task_db.clone(),
            content_moderation_handler,
        }
    };
    #[cfg(all(not(feature = "content-moderation"), feature = "video"))]
    let tasks = TaskState {
        task_queue: task_queue.clone(),
        task_repository: task_db.clone(),
        video_job_queue,
    };
    #[cfg(all(not(feature = "content-moderation"), not(feature = "video")))]
    let tasks = TaskState {
        task_queue: task_queue.clone(),
        task_repository: task_db.clone(),
    };

    let db_state = DbState {
        pool: pool.clone(),
        analytics: analytics_service,
        database: database_config,
        cleanup_service: Some(cleanup_service),
        folder_repository: folder_db,
        named_transformation_repository: named_transformation_db,
        embedding_repository: embedding_db,
        metadata_search_repository: metadata_search_db,
        api_key_repository: api_key_db,
        tenant_repository: tenant_db,
        task_repository: task_db.clone(),
        webhook_repository: webhook_db,
        webhook_event_repository: webhook_event_db,
        webhook_retry_repository: webhook_retry_db,
    };

    let webhook_state = WebhookState {
        webhook_service,
        webhook_retry_service,
    };

    let capacity_checker = Arc::new(CapacityChecker::new(config.clone()));

    #[cfg(feature = "plugin")]
    let plugins = {
        let encryption =
            mindia_core::EncryptionService::new().expect("ENCRYPTION_KEY set in setup_test_app");
        let plugin_config_repo = mindia_db::PluginConfigRepository::new_with_encryption(
            pool.clone(),
            encryption.clone(),
        );
        let plugin_execution_repo = mindia_db::PluginExecutionRepository::new(pool.clone());
        let plugin_registry = Arc::new(mindia_api::plugins::PluginRegistry::new());
        let plugin_service = mindia_api::plugins::PluginService::new_with_encryption(
            plugin_registry.clone(),
            plugin_config_repo.clone(),
            plugin_execution_repo.clone(),
            task_queue.clone(),
            encryption,
        );
        let plugin_task_handler = mindia_api::PluginTaskHandler::new(
            plugin_registry.clone(),
            plugin_config_repo.clone(),
            plugin_execution_repo.clone(),
        );
        mindia_api::state::PluginState {
            plugin_registry,
            plugin_service,
            plugin_config_repository: plugin_config_repo,
            plugin_execution_repository: plugin_execution_repo,
            plugin_task_handler,
        }
    };

    #[cfg(feature = "workflow")]
    let workflows = {
        let workflow_repo = mindia_db::WorkflowRepository::new(pool.clone());
        let workflow_execution_repo = mindia_db::WorkflowExecutionRepository::new(pool.clone());
        let workflow_service = mindia_api::WorkflowService::new(
            workflow_repo.clone(),
            workflow_execution_repo.clone(),
            task_queue.clone(),
        );
        mindia_api::state::WorkflowState {
            workflow_repository: workflow_repo,
            workflow_execution_repository: workflow_execution_repo,
            workflow_service,
        }
    };

    let state: Arc<AppState> = Arc::new(AppState {
        db: db_state,
        media: media_config,
        security: security_config,
        tasks,
        webhooks: webhook_state,
        #[cfg(feature = "plugin")]
        plugins,
        #[cfg(feature = "workflow")]
        workflows,
        config: config.clone(),
        capacity_checker,
        is_production: false,
        s3: None,
        #[cfg(feature = "semantic-search")]
        semantic_search: None,
    });

    let app = routes::setup_routes(&config, state)
        .await
        .expect("Failed to setup routes");
    let server = TestServer::new(app.into_make_service()).expect("Failed to create test server");

    TestApp {
        server,
        pool,
        _container: container,
        _temp_dir: temp_dir,
    }
}

fn create_test_config(database_url: &str) -> Config {
    use mindia_core::MediaProcessorConfig;
    let base = BaseConfig {
        server_port: 3000,
        cors_origins: vec!["*".to_string()],
        db_max_connections: 5,
        db_timeout_seconds: 30,
        jwt_secret: "test-secret-key-min-32-characters-long-for-testing".to_string(),
        jwt_expiry_hours: 24,
        http_rate_limit_per_minute: 10,
        http_tenant_rate_limit_per_minute: Some(20),
        environment: "test".to_string(),
        otel_enabled: false,
        otel_endpoint: String::new(),
        otel_service_name: "mindia-test".to_string(),
        otel_service_version: "0.1.0".to_string(),
        otel_protocol: "grpc".to_string(),
        otel_sampler: "always_on".to_string(),
        otel_sample_ratio: 1.0,
        otel_metrics_interval_secs: 30,
    };
    let media = MediaProcessorConfig {
        base,
        database_url: database_url.to_string(),
        service_api_key: None,
        storage_backend: Some(StorageBackend::Local),
        s3_bucket: None,
        s3_region: None,
        s3_endpoint: None,
        aws_region: Some("us-east-1".to_string()),
        aws_access_key_id: None,
        aws_secret_access_key: None,
        local_storage_path: Some("/tmp/mindia-test".to_string()),
        local_storage_base_url: Some("http://localhost:3000/media".to_string()),
        max_file_size_bytes: 10 * 1024 * 1024,
        allowed_extensions: vec![
            "jpg".into(),
            "jpeg".into(),
            "png".into(),
            "gif".into(),
            "webp".into(),
        ],
        allowed_content_types: vec![
            "image/jpeg".into(),
            "image/png".into(),
            "image/gif".into(),
            "image/webp".into(),
        ],
        remove_exif: false,
        max_video_size_bytes: 100 * 1024 * 1024,
        video_allowed_extensions: vec!["mp4".into(), "mov".into()],
        video_allowed_content_types: vec!["video/mp4".into(), "video/quicktime".into()],
        ffmpeg_path: "ffmpeg".to_string(),
        max_concurrent_transcodes: 1,
        hls_segment_duration: 6,
        hls_variants: vec!["360p".into(), "720p".into()],
        max_document_size_bytes: 50 * 1024 * 1024,
        document_allowed_extensions: vec!["pdf".into()],
        document_allowed_content_types: vec!["application/pdf".into()],
        max_audio_size_bytes: 20 * 1024 * 1024,
        audio_allowed_extensions: vec!["mp3".into(), "wav".into()],
        audio_allowed_content_types: vec!["audio/mpeg".into(), "audio/wav".into()],
        analytics_db_type: None,
        analytics_db_url: None,
        clamav_enabled: false,
        clamav_host: "localhost".to_string(),
        clamav_port: 3310,
        clamav_fail_closed: false,
        semantic_search_enabled: false,
        semantic_search_provider: "anthropic".to_string(),
        anthropic_api_key: None,
        voyage_api_key: None,
        anthropic_vision_model: "claude-sonnet-4-20250514".to_string(),
        voyage_embedding_model: "voyage-3-large".to_string(),
        auto_store_enabled: true,
        url_upload_allowlist: None,
        task_queue_max_workers: 2,
        task_queue_poll_interval_ms: 1000,
        task_queue_video_rate_limit: 1.0,
        task_queue_embedding_rate_limit: 5.0,
        task_queue_default_timeout_seconds: 3600,
        task_queue_max_retries: 3,
        task_queue_stale_task_reap_interval_secs: 60,
        task_queue_stale_task_grace_period_secs: 300,
        task_retention_days: 30,
        min_disk_free_gb: 1,
        max_memory_usage_percent: 90.0,
        max_cpu_usage_percent: 95.0,
        disk_check_behavior: "warn".to_string(),
        memory_check_behavior: "warn".to_string(),
        cpu_check_behavior: "warn".to_string(),
        video_transcode_space_multiplier: 4.0,
        capacity_monitor_interval_secs: 5,
        capacity_monitor_enabled: false,
        email_alerts_enabled: false,
        smtp_host: None,
        smtp_port: None,
        smtp_user: None,
        smtp_password: None,
        smtp_from: None,
        smtp_tls: true,
        frontend_url: None,
    };
    Config(Box::new(media))
}
