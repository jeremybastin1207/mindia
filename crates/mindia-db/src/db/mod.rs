//! Database repositories for data access layer
//!
//! This module contains all repository implementations for database operations.
//! Repositories are organized into control/ (auth, tenants, webhooks, billing) and media/
//! (media operations, processing, storage). Each repository is responsible for a specific
//! domain entity and provides CRUD operations and specialized queries.
//
// Core repositories (API keys, users, tenants, billing, webhooks, etc.)
pub mod control;
//
// Media repositories (media operations, processing, storage, etc.)
pub mod media;
//
// Transaction utilities
pub mod transaction;
//
// Re-export from media processor
#[cfg(feature = "semantic-search")]
pub use media::EmbeddingRepository;
pub use media::{
    AnalyticsRepository, FileGroupRepository, FolderRepository, MediaRepository,
    MediaTenantRepository, MetadataSearchRepository, NamedTransformationRepository,
    PluginConfigRepository, PluginExecutionRepository, PresignedUploadRepository,
    StorageLocationRepository, StorageMetricsRepository, TaskRepository,
};
//
// Analytics repository factory and trait (from media::analytics)
pub use media::analytics::{create_analytics_repository, AnalyticsRepositoryTrait};
//
// Re-exports for control/ repositories and helpers
pub use control::{
    calculate_next_retry_time, ApiKeyRepository, TenantRepository, WebhookEventRepository,
    WebhookRepository, WebhookRetryRepository,
};
