use crate::job_queue::VideoJobQueue;
#[cfg(feature = "plugin")]
use crate::plugins::{PluginRegistry, PluginService};
use crate::task_handlers::{ContentModerationTaskHandler, PluginTaskHandler};
use mindia_core::Config;
use mindia_db::{
    ApiKeyRepository, EmbeddingRepository, FileGroupRepository, FolderRepository, MediaRepository,
    MetadataSearchRepository, NamedTransformationRepository, TaskRepository, TenantRepository,
    WebhookEventRepository, WebhookRepository, WebhookRetryRepository,
};
#[cfg(feature = "plugin")]
use mindia_db::{PluginConfigRepository, PluginExecutionRepository};
use mindia_infra::{
    AnalyticsService, CapacityChecker, CleanupService, WebhookRetryService, WebhookService,
};
#[cfg(feature = "clamav")]
use mindia_services::ClamAVService;
#[cfg(feature = "semantic-search")]
use mindia_services::SemanticSearchProvider;
use mindia_services::{S3Service, Storage};
use mindia_worker::TaskQueue;
use sqlx::PgPool;
use std::sync::Arc;

/// Image-specific configuration and services
#[derive(Clone)]
pub struct ImageConfig {
    pub repository: MediaRepository,
    pub max_file_size: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
    #[allow(dead_code)]
    pub remove_exif: bool,
}

/// Video-specific configuration and services
#[cfg(feature = "video")]
#[derive(Clone)]
pub struct VideoConfig {
    pub repository: MediaRepository,
    #[allow(dead_code)]
    pub job_queue: VideoJobQueue,
    pub max_file_size: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
    #[allow(dead_code)]
    pub ffmpeg_path: String,
    #[allow(dead_code)]
    pub hls_segment_duration: u64,
    #[allow(dead_code)]
    pub hls_variants: Vec<String>,
}

/// Document-specific configuration and services
#[cfg(feature = "document")]
#[derive(Clone)]
pub struct DocumentConfig {
    pub repository: MediaRepository,
    pub max_file_size: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
}

/// Audio-specific configuration and services
#[cfg(feature = "audio")]
#[derive(Clone)]
pub struct AudioConfig {
    pub repository: MediaRepository,
    pub max_file_size: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
}

/// Unified media configuration and repository for all media types.
#[derive(Clone)]
pub struct MediaConfig {
    pub repository: MediaRepository,
    pub file_group_repository: FileGroupRepository,
    pub storage: Arc<dyn Storage>,

    // Image settings
    pub image_max_file_size: usize,
    pub image_allowed_extensions: Vec<String>,
    pub image_allowed_content_types: Vec<String>,
    pub remove_exif: bool,

    // Video settings
    pub video_max_file_size: usize,
    pub video_allowed_extensions: Vec<String>,
    pub video_allowed_content_types: Vec<String>,
    pub ffmpeg_path: String,
    pub hls_segment_duration: u64,
    pub hls_variants: Vec<String>,

    // Audio settings
    #[allow(dead_code)]
    pub audio_max_file_size: usize,
    #[allow(dead_code)]
    pub audio_allowed_extensions: Vec<String>,
    #[allow(dead_code)]
    pub audio_allowed_content_types: Vec<String>,

    // Document settings
    #[allow(dead_code)]
    pub document_max_file_size: usize,
    #[allow(dead_code)]
    pub document_allowed_extensions: Vec<String>,
    #[allow(dead_code)]
    pub document_allowed_content_types: Vec<String>,
}

/// S3 storage configuration
#[derive(Clone)]
pub struct S3Config {
    pub service: S3Service,
    pub bucket: String,
    pub region: String,
    pub endpoint_url: Option<String>, // Custom endpoint for S3-compatible providers
}

/// Security configuration
#[derive(Clone)]
pub struct SecurityConfig {
    #[cfg(feature = "clamav")]
    pub clamav: Option<ClamAVService>,
    pub clamav_enabled: bool,
    pub cors_origins: Vec<String>,
}

/// Database configuration
#[derive(Clone)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub timeout_seconds: u64,
}

/// Main application state with organized sub-modules
#[derive(Clone)]
pub struct AppState {
    // Database pool (for health checks and direct queries)
    pub db_pool: PgPool,

    /// Unified media configuration and repository for all media types.
    pub media: MediaConfig,

    /// Image-specific configuration (file size limits, extensions, etc.).
    pub image: ImageConfig,

    /// Video-specific configuration and job queue.
    #[cfg(feature = "video")]
    pub video: VideoConfig,

    /// Document-specific configuration.
    #[cfg(feature = "document")]
    pub document: DocumentConfig,

    /// Audio-specific configuration.
    #[cfg(feature = "audio")]
    pub audio: AudioConfig,

    pub s3: Option<S3Config>,
    pub security: SecurityConfig,
    pub database: DatabaseConfig,
    pub analytics: AnalyticsService,
    pub is_production: bool,
    #[cfg(feature = "semantic-search")]
    pub semantic_search: Option<std::sync::Arc<dyn SemanticSearchProvider + Send + Sync>>,
    pub embedding_repository: EmbeddingRepository,
    pub metadata_search_repository: MetadataSearchRepository,
    pub cleanup_service: Option<CleanupService>,
    pub config: Config,
    // New centralized task management
    pub task_queue: TaskQueue,
    pub task_repository: TaskRepository,
    #[cfg(feature = "video")]
    pub video_db: MediaRepository,
    // Webhook system
    #[allow(dead_code)]
    pub webhook_repository: WebhookRepository,
    #[allow(dead_code)]
    pub webhook_event_repository: WebhookEventRepository,
    #[allow(dead_code)]
    pub webhook_retry_repository: WebhookRetryRepository,
    pub webhook_service: WebhookService,
    #[allow(dead_code)]
    pub webhook_retry_service: WebhookRetryService,
    pub folder_repository: FolderRepository,
    // API keys and tenants (for auth with generated keys)
    pub api_key_repository: ApiKeyRepository,
    pub tenant_repository: TenantRepository,
    // Named transformations (presets)
    pub named_transformation_repository: NamedTransformationRepository,
    // Plugin system
    #[cfg(feature = "plugin")]
    #[allow(dead_code)]
    pub plugin_registry: Arc<PluginRegistry>,
    #[cfg(feature = "plugin")]
    pub plugin_service: PluginService,
    #[cfg(feature = "plugin")]
    #[allow(dead_code)]
    pub plugin_config_repository: PluginConfigRepository,
    #[cfg(feature = "plugin")]
    #[allow(dead_code)]
    pub plugin_execution_repository: PluginExecutionRepository,
    #[cfg(feature = "plugin")]
    pub plugin_task_handler: PluginTaskHandler,
    #[cfg(feature = "content-moderation")]
    pub content_moderation_handler: ContentModerationTaskHandler,
    pub capacity_checker: Arc<CapacityChecker>,
}

#[allow(dead_code)]
fn _assert_app_state_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<AppState>();
    assert_sync::<AppState>();
}
