//! OpenAPI documentation.
//! API version is in `crate::constants::API_VERSION`.
//! Paths in handler annotations use placeholder /api/v0; they are transformed at runtime to the actual version.

use utoipa::OpenApi;

use crate::constants::API_VERSION;
use crate::error;
use crate::handlers;
use mindia_core::models;

/// Placeholder version used in handler path annotations (utoipa requires compile-time literals).
/// Replaced at runtime in the served OpenAPI spec with API_VERSION.
const OPENAPI_PATH_PLACEHOLDER: &str = "/api/v0";

/// Transforms path keys in the OpenAPI spec from placeholder to actual API version.
fn transform_openapi_paths(spec: &mut utoipa::openapi::OpenApi, version: &str) {
    let replacement = format!("/api/{}", version);
    if OPENAPI_PATH_PLACEHOLDER == replacement {
        return;
    }
    let path_map = std::mem::take(&mut spec.paths.paths);
    for (key, item) in path_map {
        let new_key = key.replacen(OPENAPI_PATH_PLACEHOLDER, &replacement, 1);
        spec.paths.paths.insert(new_key, item);
    }
}

/// Returns the OpenAPI spec with path placeholders replaced by the current API version.
pub fn get_openapi_spec() -> utoipa::openapi::OpenApi {
    let mut spec = ApiDoc::openapi();
    transform_openapi_paths(&mut spec, API_VERSION);
    spec
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Mindia API",
        version = "0.1.0",
        description = "Media management API (v0) with support for images, videos, and documents. Features include S3 storage, HLS video streaming, image transformations, and semantic search. All endpoints are versioned under /api/v0/.",
        contact(
            name = "API Support",
            url = "https://github.com/yourusername/mindia"
        )
    ),
    paths(
        // Images
        handlers::image_upload::upload_image,
        handlers::image_get::get_image,
        handlers::image_get::list_images,
        handlers::image_download::download_image,
        handlers::transform::transform_image,
        handlers::metadata::update_image_metadata,
        // Videos
        handlers::video_upload::upload_video,
        handlers::video_get::get_video,
        handlers::video_get::list_videos,
        handlers::metadata::update_video_metadata,
        handlers::video_stream::stream_master_playlist,
        handlers::video_stream::stream_variant_playlist,
        handlers::video_stream::stream_segment,
        // Documents
        handlers::document_upload::upload_document,
        handlers::document_get::get_document,
        handlers::document_get::list_documents,
        handlers::document_download::download_document,
        handlers::metadata::update_document_metadata,
        // Audio
        handlers::audio_upload::upload_audio,
        handlers::audio_get::get_audio,
        handlers::audio_get::list_audios,
        handlers::audio_download::download_audio,
        handlers::metadata::update_audio_metadata,
        // Media (unified operations)
        handlers::media_get::get_media,
        handlers::media_delete::delete_media,
        handlers::batch_media::batch_delete_media,
        handlers::batch_media::batch_copy_media,
        // Uploads (Presigned & Chunked)
        handlers::presigned_upload::generate_presigned_url,
        handlers::presigned_upload::complete_upload,
        handlers::chunked_upload::start_chunked_upload,
        handlers::chunked_upload::record_chunk_upload,
        handlers::chunked_upload::complete_chunked_upload,
        handlers::chunked_upload::get_chunked_upload_progress,
        // Metadata
        handlers::metadata::get_metadata,
        // Folders
        handlers::folders::create_folder,
        handlers::folders::list_folders,
        handlers::folders::get_folder_tree,
        handlers::folders::get_folder,
        handlers::folders::update_folder,
        handlers::folders::delete_folder,
        handlers::folders::move_media,
        // Analytics
        handlers::analytics::get_traffic_summary,
        handlers::analytics::get_url_statistics,
        handlers::analytics::get_storage_summary,
        handlers::analytics::refresh_storage_metrics,
        handlers::analytics::list_audit_logs,
        handlers::analytics::get_audit_log,
        // Search
        handlers::search::search_files,
        // File groups
        handlers::file_group::create_file_group,
        handlers::file_group::get_file_group,
        handlers::file_group::get_file_group_info,
        handlers::file_group::get_file_by_index,
        handlers::file_group::get_group_archive,
        handlers::file_group::delete_file_group,
        // Config
        handlers::config::get_config,
        // Batch operations
        handlers::batch::batch_operations,
    ),
    components(
        schemas(
            // Core models
            models::ImageResponse,
            models::VideoResponse,
            models::ProcessingStatus,
            models::DocumentResponse,
            // Folder models
            models::FolderResponse,
            models::CreateFolderRequest,
            models::UpdateFolderRequest,
            models::FolderTreeNode,
            handlers::folders::FolderListQuery,
            handlers::folders::MoveMediaRequest,
            // Analytics models
            models::TrafficSummary,
            models::UrlStatistics,
            models::StorageSummary,
            models::ContentTypeStats,
            models::AnalyticsQuery,
            // Search models
            models::SearchResult,
            models::SearchQuery,
            models::EntityType,
            // File group models
            models::CreateFileGroupRequest,
            models::FileGroupInfo,
            models::FileGroupResponse,
            models::FileGroupFileItem,
            // Query params
            handlers::image_get::PaginationQuery,
            handlers::video_get::ListQuery,
            handlers::document_get::PaginationQuery,
            handlers::search::SearchResponse,
            // Config
            handlers::config::ConfigResponse,
            handlers::config::S3Config,
            handlers::config::UploadConfig,
            handlers::config::DatabaseConfig,
            handlers::config::CorsConfig,
            handlers::config::ClamAVConfig,
            // Upload models
            models::PresignedUploadRequest,
            models::PresignedUploadResponse,
            models::CompleteUploadRequest,
            models::CompleteUploadResponse,
            handlers::chunked_upload::StartChunkedUploadRequest,
            handlers::chunked_upload::StartChunkedUploadResponse,
            handlers::chunked_upload::ChunkUrl,
            handlers::chunked_upload::UploadChunkRequest,
            handlers::chunked_upload::ChunkedUploadProgressResponse,
            // Metadata models
            handlers::metadata::UpdateMetadataRequest,
            handlers::metadata::MetadataResponse,
            // Batch models
            handlers::batch::BatchRequest,
            handlers::batch::BatchResponse,
            handlers::batch::BatchOperation,
            handlers::batch::BatchResult,
            handlers::batch_media::BatchMediaRequest,
            handlers::batch_media::BatchDeleteResponse,
            handlers::batch_media::BatchDeleteResult,
            handlers::batch_media::BatchCopyResponse,
            handlers::batch_media::BatchCopyResult,
            // Error
            error::ErrorResponse,
        )
    ),
    tags(
        (name = "images", description = "Image upload, management, and transformation operations"),
        (name = "videos", description = "Video upload, management, and HLS streaming operations"),
        (name = "documents", description = "Document upload, management, and download operations"),
        (name = "folders", description = "Folder management and media organization operations"),
        (name = "analytics", description = "Traffic and storage analytics"),
        (name = "search", description = "Semantic search for files"),
        (name = "file-groups", description = "File group operations for organizing multiple files"),
        (name = "config", description = "Service configuration and health checks"),
        (name = "uploads", description = "Presigned URL and chunked upload operations"),
        (name = "media", description = "Unified media operations (delete, metadata, etc.)"),
        (name = "batch", description = "Batch operations for executing multiple API calls in a single request")
    )
)]
pub struct ApiDoc;
