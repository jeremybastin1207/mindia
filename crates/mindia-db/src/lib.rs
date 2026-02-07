//! Mindia Database Layer (OSS)
//!
//! This crate provides database repositories and data access functionality
//! for the open-source media/data plane.
//!
// Module declarations
pub mod db;
pub mod plugin_traits;

// Re-exports: Core repositories (auth, tenants, webhooks) and helpers
pub use db::{
    calculate_next_retry_time, ApiKeyRepository, TenantRepository, WebhookEventRepository,
    WebhookRepository, WebhookRetryRepository,
};

// Re-exports: Media repositories and factories
pub use db::{
    create_analytics_repository, AnalyticsRepository, AnalyticsRepositoryTrait,
    FileGroupRepository, FolderRepository, MediaRepository, MediaTenantRepository,
    MetadataSearchRepository, NamedTransformationRepository, PluginConfigRepository,
    PluginExecutionRepository, PresignedUploadRepository, StorageLocationRepository,
    StorageMetricsRepository, TaskRepository,
};

// Re-exports: Transaction utilities
pub use db::transaction::{with_transaction, TransactionGuard};

// Re-exports: Module convenience re-exports
pub use db::media;

// Re-exports: Plugin traits
pub use plugin_traits::{PluginFileGroupRepository, PluginMediaRepository};

#[cfg(feature = "semantic-search")]
pub use db::EmbeddingRepository;
