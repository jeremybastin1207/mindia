pub mod analytics;
#[cfg(feature = "semantic-search")]
pub mod embedding;
pub mod file_group;
pub mod folder;
#[allow(clippy::module_inception)]
pub mod media;
pub mod metadata_search;
pub mod named_transformation;
pub mod plugin;
pub mod presigned_upload;
pub mod storage;
pub mod task;
pub mod tenant;
pub mod workflow;

pub use analytics::{
    create_analytics_repository, AnalyticsRepositoryTrait,
    PostgresAnalyticsRepository as AnalyticsRepository, StorageMetricsRepository,
};
#[cfg(feature = "semantic-search")]
pub use embedding::EmbeddingRepository;
pub use file_group::FileGroupRepository;
pub use folder::FolderRepository;
pub use media::MediaRepository;
pub use metadata_search::MetadataSearchRepository;
pub use named_transformation::NamedTransformationRepository;
pub use plugin::{PluginConfigRepository, PluginExecutionRepository};
pub use presigned_upload::PresignedUploadRepository;
pub use storage::StorageLocationRepository;
pub use task::TaskRepository;
pub use tenant::TenantRepository;
pub use workflow::{WorkflowExecutionRepository, WorkflowRepository};
