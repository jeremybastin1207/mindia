//! Application state and sub-state extractors.
//!
//! AppState is split into domain sub-states so handlers can extract only what they need
//! via Axum's `FromRef`, and to avoid a single god object with duplicate repositories.

#[cfg(feature = "content-moderation")]
use crate::task_handlers::ContentModerationTaskHandler;
use crate::task_handlers::PluginTaskHandler;
use mindia_core::models::MediaType;
use mindia_core::Config;
use mindia_db::{
    ApiKeyRepository, EmbeddingRepository, FileGroupRepository, FolderRepository, MediaRepository,
    MetadataSearchRepository, NamedTransformationRepository, TaskRepository, TenantRepository,
    WebhookEventRepository, WebhookRepository, WebhookRetryRepository,
};
use mindia_infra::{
    AnalyticsService, CapacityChecker, CleanupService, WebhookRetryService, WebhookService,
};
use mindia_services::Storage;
use mindia_worker::TaskQueue;
use sqlx::PgPool;
use std::sync::Arc;

#[cfg(feature = "plugin")]
use crate::plugins::{PluginRegistry, PluginService};
#[cfg(feature = "workflow")]
use crate::services::workflow::WorkflowService;
#[cfg(feature = "plugin")]
use mindia_db::{PluginConfigRepository, PluginExecutionRepository};
#[cfg(feature = "workflow")]
use mindia_db::{WorkflowExecutionRepository, WorkflowRepository};
#[cfg(feature = "clamav")]
use mindia_services::ClamAVService;
use mindia_services::S3Service;
#[cfg(feature = "semantic-search")]
use mindia_services::SemanticSearchProvider;

// ----- Sub-state types -----

/// Database pool, analytics, cleanup, and all repositories that are not tied to a specific service.
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services; not all fields referenced in every build
pub struct DbState {
    pub pool: PgPool,
    pub analytics: AnalyticsService,
    pub database: DatabaseConfig,
    pub cleanup_service: Option<CleanupService>,
    pub folder_repository: FolderRepository,
    pub named_transformation_repository: NamedTransformationRepository,
    pub embedding_repository: EmbeddingRepository,
    pub metadata_search_repository: MetadataSearchRepository,
    pub api_key_repository: ApiKeyRepository,
    pub tenant_repository: TenantRepository,
    pub task_repository: TaskRepository,
    pub webhook_repository: WebhookRepository,
    pub webhook_event_repository: WebhookEventRepository,
    pub webhook_retry_repository: WebhookRetryRepository,
}

/// Unified media configuration and repository for all media types.
#[derive(Clone)]
pub struct MediaConfig {
    pub repository: MediaRepository,
    pub file_group_repository: FileGroupRepository,
    pub storage: Arc<dyn Storage>,
    pub image_max_file_size: usize,
    pub image_allowed_extensions: Vec<String>,
    pub image_allowed_content_types: Vec<String>,
    pub remove_exif: bool,
    pub video_max_file_size: usize,
    pub video_allowed_extensions: Vec<String>,
    pub video_allowed_content_types: Vec<String>,
    pub ffmpeg_path: String,
    pub hls_segment_duration: u64,
    pub hls_variants: Vec<String>,
    pub audio_max_file_size: usize,
    pub audio_allowed_extensions: Vec<String>,
    pub audio_allowed_content_types: Vec<String>,
    pub document_max_file_size: usize,
    pub document_allowed_extensions: Vec<String>,
    pub document_allowed_content_types: Vec<String>,
}

/// Limits and allowlists for a single media type (from MediaConfig).
#[derive(Clone, Debug)]
pub struct MediaLimits {
    pub max_file_size: usize,
    pub allowed_extensions: Vec<String>,
    pub allowed_content_types: Vec<String>,
}

impl MediaConfig {
    /// Return size limits and allowlists for the given media type.
    pub fn limits_for(&self, media_type: MediaType) -> MediaLimits {
        match media_type {
            MediaType::Image => MediaLimits {
                max_file_size: self.image_max_file_size,
                allowed_extensions: self.image_allowed_extensions.clone(),
                allowed_content_types: self.image_allowed_content_types.clone(),
            },
            MediaType::Video => MediaLimits {
                max_file_size: self.video_max_file_size,
                allowed_extensions: self.video_allowed_extensions.clone(),
                allowed_content_types: self.video_allowed_content_types.clone(),
            },
            MediaType::Audio => MediaLimits {
                max_file_size: self.audio_max_file_size,
                allowed_extensions: self.audio_allowed_extensions.clone(),
                allowed_content_types: self.audio_allowed_content_types.clone(),
            },
            MediaType::Document => MediaLimits {
                max_file_size: self.document_max_file_size,
                allowed_extensions: self.document_allowed_extensions.clone(),
                allowed_content_types: self.document_allowed_content_types.clone(),
            },
        }
    }
}

/// S3 storage configuration (for chunked uploads).
#[derive(Clone)]
pub struct S3Config {
    pub service: S3Service,
    pub bucket: String,
    pub region: String,
    pub endpoint_url: Option<String>,
}

/// Security configuration (ClamAV, CORS).
#[cfg(feature = "clamav")]
#[derive(Clone)]
pub struct SecurityConfig {
    pub clamav: Option<ClamAVService>,
    pub clamav_enabled: bool,
    pub cors_origins: Vec<String>,
}

#[derive(Clone)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub timeout_seconds: u64,
}

/// Task queue and related handlers.
#[cfg(all(feature = "content-moderation", feature = "video"))]
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services; variant depends on features
pub struct TaskState {
    pub task_queue: TaskQueue,
    pub task_repository: TaskRepository,
    pub content_moderation_handler: ContentModerationTaskHandler,
    pub video_job_queue: crate::job_queue::VideoJobQueue,
}

#[cfg(all(feature = "content-moderation", not(feature = "video")))]
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services
pub struct TaskState {
    pub task_queue: TaskQueue,
    pub task_repository: TaskRepository,
    pub content_moderation_handler: ContentModerationTaskHandler,
}

#[cfg(all(not(feature = "content-moderation"), feature = "video"))]
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services
pub struct TaskState {
    pub task_queue: TaskQueue,
    pub task_repository: TaskRepository,
    pub video_job_queue: crate::job_queue::VideoJobQueue,
}

#[cfg(all(not(feature = "content-moderation"), not(feature = "video")))]
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services
pub struct TaskState {
    pub task_queue: TaskQueue,
    pub task_repository: TaskRepository,
}

/// Webhook delivery and retry services.
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services; not all fields in every code path
pub struct WebhookState {
    pub webhook_service: WebhookService,
    pub webhook_retry_service: WebhookRetryService,
}

#[cfg(feature = "plugin")]
#[derive(Clone)]
#[allow(dead_code)] // Used via FromRef and in setup::services
pub struct PluginState {
    pub plugin_registry: Arc<PluginRegistry>,
    pub plugin_service: PluginService,
    pub plugin_config_repository: PluginConfigRepository,
    pub plugin_execution_repository: PluginExecutionRepository,
    pub plugin_task_handler: PluginTaskHandler,
}

#[cfg(feature = "workflow")]
#[derive(Clone)]
pub struct WorkflowState {
    pub workflow_repository: WorkflowRepository,
    pub workflow_execution_repository: WorkflowExecutionRepository,
    pub workflow_service: WorkflowService,
}

#[cfg(not(feature = "clamav"))]
#[derive(Clone)]
pub struct SecurityConfig {
    pub clamav_enabled: bool,
    pub cors_origins: Vec<String>,
}

// ----- AppState -----

/// Main application state: aggregates sub-states for dependency injection.
#[derive(Clone)]
pub struct AppState {
    pub db: DbState,
    pub media: MediaConfig,
    pub security: SecurityConfig,
    pub tasks: TaskState,
    pub webhooks: WebhookState,
    #[cfg(feature = "plugin")]
    pub plugins: PluginState,
    #[cfg(feature = "workflow")]
    pub workflows: WorkflowState,
    pub config: Config,
    pub capacity_checker: Arc<CapacityChecker>,
    pub is_production: bool,
    pub s3: Option<S3Config>,
    #[cfg(feature = "semantic-search")]
    pub semantic_search: Option<Arc<dyn SemanticSearchProvider + Send + Sync>>,
}

// ----- FromRef for sub-state extraction -----

impl axum::extract::FromRef<Arc<AppState>> for DbState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.db.clone()
    }
}

impl axum::extract::FromRef<Arc<AppState>> for MediaConfig {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.media.clone()
    }
}

impl axum::extract::FromRef<Arc<AppState>> for SecurityConfig {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.security.clone()
    }
}

impl axum::extract::FromRef<Arc<AppState>> for TaskState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.tasks.clone()
    }
}

impl axum::extract::FromRef<Arc<AppState>> for WebhookState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.webhooks.clone()
    }
}

#[cfg(feature = "plugin")]
impl axum::extract::FromRef<Arc<AppState>> for PluginState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.plugins.clone()
    }
}

#[cfg(feature = "workflow")]
impl axum::extract::FromRef<Arc<AppState>> for WorkflowState {
    fn from_ref(state: &Arc<AppState>) -> Self {
        state.workflows.clone()
    }
}

fn _assert_app_state_send_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    assert_send::<AppState>();
    assert_sync::<AppState>();
}
