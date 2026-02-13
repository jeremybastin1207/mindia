//! Domain route groups (media, images, videos, documents, etc.).

use crate::constants::API_PREFIX;
use crate::handlers;
use crate::state::AppState;
use axum::routing::{delete, get, post, put};
use axum::Router;
use std::sync::Arc;

pub fn media_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{}/media/:id", API_PREFIX), get(handlers::media_get::get_media))
        .route(&format!("{}/media/:id", API_PREFIX), delete(handlers::media_delete::delete_media))
        .route(
            &format!("{}/media/batch/delete", API_PREFIX),
            post(handlers::batch_media::batch_delete_media),
        )
        .route(
            &format!("{}/media/batch/copy", API_PREFIX),
            post(handlers::batch_media::batch_copy_media),
        )
        .with_state(state)
}

pub fn image_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{}/images", API_PREFIX), post(handlers::image_upload::upload_image))
        .route(
            &format!("{}/images/from-url", API_PREFIX),
            post(handlers::image_upload_url::upload_image_from_url),
        )
        .route(&format!("{}/images", API_PREFIX), get(handlers::image_get::list_images))
        .route(&format!("{}/images/:id", API_PREFIX), get(handlers::image_get::get_image))
        .route(
            &format!("{}/images/:id/file", API_PREFIX),
            get(handlers::image_download::download_image),
        )
        .route(
            &format!("{}/images/:id/*operations", API_PREFIX),
            get(handlers::transform::transform_image),
        )
        .route(
            &format!("{}/images/:id/metadata", API_PREFIX),
            put(handlers::metadata::update_image_metadata),
        )
        .with_state(state)
}

pub fn video_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "video")]
    {
        Router::new()
            .route(&format!("{}/videos", API_PREFIX), post(handlers::video_upload::upload_video))
            .route(&format!("{}/videos", API_PREFIX), get(handlers::video_get::list_videos))
            .route(&format!("{}/videos/:id", API_PREFIX), get(handlers::video_get::get_video))
            .route(
                &format!("{}/videos/:id/metadata", API_PREFIX),
                put(handlers::metadata::update_video_metadata),
            )
            .route(
                &format!("{}/videos/:id/stream/master.m3u8", API_PREFIX),
                get(handlers::video_stream::stream_master_playlist),
            )
            .route(
                &format!("{}/videos/:id/stream/:variant/index.m3u8", API_PREFIX),
                get(handlers::video_stream::stream_variant_playlist),
            )
            .route(
                &format!("{}/videos/:id/stream/:variant/:segment", API_PREFIX),
                get(handlers::video_stream::stream_segment),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "video"))]
    {
        Router::new().with_state(state)
    }
}

pub fn document_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "document")]
    {
        Router::new()
            .route(
                &format!("{}/documents", API_PREFIX),
                post(handlers::document_upload::upload_document),
            )
            .route(&format!("{}/documents", API_PREFIX), get(handlers::document_get::list_documents))
            .route(
                &format!("{}/documents/:id", API_PREFIX),
                get(handlers::document_get::get_document),
            )
            .route(
                &format!("{}/documents/:id/file", API_PREFIX),
                get(handlers::document_download::download_document),
            )
            .route(
                &format!("{}/documents/:id/metadata", API_PREFIX),
                put(handlers::metadata::update_document_metadata),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "document"))]
    {
        Router::new().with_state(state)
    }
}

pub fn audio_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "audio")]
    {
        Router::new()
            .route(&format!("{}/audios", API_PREFIX), post(handlers::audio_upload::upload_audio))
            .route(&format!("{}/audios", API_PREFIX), get(handlers::audio_get::list_audios))
            .route(&format!("{}/audios/:id", API_PREFIX), get(handlers::audio_get::get_audio))
            .route(
                &format!("{}/audios/:id/file", API_PREFIX),
                get(handlers::audio_download::download_audio),
            )
            .route(
                &format!("{}/audios/:id/metadata", API_PREFIX),
                put(handlers::metadata::update_audio_metadata),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "audio"))]
    {
        Router::new().with_state(state)
    }
}

pub fn folder_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{}/folders", API_PREFIX), post(handlers::folders::create_folder))
        .route(&format!("{}/folders", API_PREFIX), get(handlers::folders::list_folders))
        .route(&format!("{}/folders/tree", API_PREFIX), get(handlers::folders::get_folder_tree))
        .route(&format!("{}/folders/:id", API_PREFIX), get(handlers::folders::get_folder))
        .route(&format!("{}/folders/:id", API_PREFIX), put(handlers::folders::update_folder))
        .route(&format!("{}/folders/:id", API_PREFIX), delete(handlers::folders::delete_folder))
        .route(
            &format!("{}/media/:id/folder", API_PREFIX),
            put(handlers::folders::move_media),
        )
        .with_state(state)
}

pub fn preset_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/presets", API_PREFIX),
            post(handlers::named_transformations::create_preset)
                .get(handlers::named_transformations::list_presets),
        )
        .route(
            &format!("{}/presets/:name", API_PREFIX),
            get(handlers::named_transformations::get_preset)
                .put(handlers::named_transformations::update_preset)
                .delete(handlers::named_transformations::delete_preset),
        )
        .with_state(state)
}

pub fn analytics_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/analytics/traffic", API_PREFIX),
            get(handlers::analytics::get_traffic_summary),
        )
        .route(
            &format!("{}/analytics/urls", API_PREFIX),
            get(handlers::analytics::get_url_statistics),
        )
        .route(
            &format!("{}/analytics/storage", API_PREFIX),
            get(handlers::analytics::get_storage_summary),
        )
        .route(
            &format!("{}/analytics/storage/refresh", API_PREFIX),
            post(handlers::analytics::refresh_storage_metrics),
        )
        .route(
            &format!("{}/audit-logs", API_PREFIX),
            get(handlers::analytics::list_audit_logs),
        )
        .route(
            &format!("{}/audit-logs/:id", API_PREFIX),
            get(handlers::analytics::get_audit_log),
        )
        .with_state(state)
}

pub fn search_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{}/search", API_PREFIX), get(handlers::search::search_files))
        .with_state(state)
}

pub fn metadata_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{}/config", API_PREFIX), get(handlers::config::get_config))
        .route(
            &format!("{}/media/:id/metadata", API_PREFIX),
            get(handlers::metadata::get_metadata),
        )
        .route(
            &format!("{}/media/:id/metadata/:key", API_PREFIX),
            put(handlers::metadata::update_metadata_key)
                .get(handlers::metadata::get_metadata_key)
                .delete(handlers::metadata::delete_metadata_key),
        )
        .with_state(state)
}

pub fn task_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{}/tasks", API_PREFIX), get(handlers::tasks::list_tasks))
        .route(&format!("{}/tasks/:id", API_PREFIX), get(handlers::tasks::get_task))
        .route(
            &format!("{}/tasks/:id/cancel", API_PREFIX),
            post(handlers::tasks::cancel_task),
        )
        .route(
            &format!("{}/tasks/:id/retry", API_PREFIX),
            post(handlers::tasks::retry_task),
        )
        .route(
            &format!("{}/tasks/stats", API_PREFIX),
            get(handlers::tasks::get_task_stats),
        )
        .with_state(state)
}

pub fn file_group_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/groups", API_PREFIX),
            post(handlers::file_group::create_file_group),
        )
        .route(
            &format!("{}/groups/:id", API_PREFIX),
            get(handlers::file_group::get_file_group),
        )
        .route(
            &format!("{}/groups/:id/info", API_PREFIX),
            get(handlers::file_group::get_file_group_info),
        )
        .route(
            &format!("{}/groups/:id/nth/:index", API_PREFIX),
            get(handlers::file_group::get_file_by_index),
        )
        .route(
            &format!("{}/groups/:id/archive/:format", API_PREFIX),
            get(handlers::file_group::get_group_archive),
        )
        .route(
            &format!("{}/groups/:id", API_PREFIX),
            delete(handlers::file_group::delete_file_group),
        )
        .with_state(state)
}

pub fn webhook_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/webhooks", API_PREFIX),
            post(handlers::webhooks::create_webhook).get(handlers::webhooks::list_webhooks),
        )
        .route(
            &format!("{}/webhooks/:id", API_PREFIX),
            get(handlers::webhooks::get_webhook)
                .put(handlers::webhooks::update_webhook)
                .delete(handlers::webhooks::delete_webhook),
        )
        .route(
            &format!("{}/webhooks/:id/events", API_PREFIX),
            get(handlers::webhooks::list_webhook_events),
        )
        .with_state(state)
}

pub fn api_key_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/api-keys", API_PREFIX),
            post(handlers::api_keys::create_api_key).get(handlers::api_keys::list_api_keys),
        )
        .route(
            &format!("{}/api-keys/:id", API_PREFIX),
            get(handlers::api_keys::get_api_key).delete(handlers::api_keys::revoke_api_key),
        )
        .with_state(state)
}

pub fn plugin_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    #[cfg(feature = "plugin")]
    {
        Router::new()
            .route(&format!("{}/plugins", API_PREFIX), get(handlers::plugins::list_plugins))
            .route(
                &format!("{}/plugins/costs", API_PREFIX),
                get(handlers::plugins::get_plugin_costs),
            )
            .route(
                &format!("{}/plugins/costs/summary", API_PREFIX),
                get(handlers::plugins::get_plugin_costs_summary),
            )
            .route(
                &format!("{}/plugins/:plugin_name/execute", API_PREFIX),
                post(handlers::plugins::execute_plugin),
            )
            .route(
                &format!("{}/plugins/:plugin_name/config", API_PREFIX),
                get(handlers::plugins::get_plugin_config),
            )
            .route(
                &format!("{}/plugins/:plugin_name/config", API_PREFIX),
                put(handlers::plugins::update_plugin_config),
            )
            .route(
                &format!("{}/plugins/:plugin_name/costs", API_PREFIX),
                get(handlers::plugins::get_plugin_costs_by_name),
            )
            .with_state(state)
    }
    #[cfg(not(feature = "plugin"))]
    {
        Router::new().with_state(state)
    }
}

#[cfg(feature = "workflow")]
pub fn workflow_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/workflows", API_PREFIX),
            post(handlers::workflows::create_workflow),
        )
        .route(
            &format!("{}/workflows", API_PREFIX),
            get(handlers::workflows::list_workflows),
        )
        .route(
            &format!("{}/workflows/:id", API_PREFIX),
            get(handlers::workflows::get_workflow),
        )
        .route(
            &format!("{}/workflows/:id", API_PREFIX),
            put(handlers::workflows::update_workflow),
        )
        .route(
            &format!("{}/workflows/:id", API_PREFIX),
            delete(handlers::workflows::delete_workflow),
        )
        .route(
            &format!("{}/workflows/:id/trigger/:media_id", API_PREFIX),
            post(handlers::workflows::trigger_workflow),
        )
        .route(
            &format!("{}/workflows/:id/executions", API_PREFIX),
            get(handlers::workflows::list_workflow_executions),
        )
        .route(
            &format!("{}/workflow-executions/:id", API_PREFIX),
            get(handlers::workflows::get_workflow_execution),
        )
        .with_state(state)
}

#[cfg(not(feature = "workflow"))]
pub fn workflow_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new().with_state(state)
}

pub fn upload_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            &format!("{}/uploads/chunked/start", API_PREFIX),
            post(handlers::chunked_upload::start_chunked_upload),
        )
        .route(
            &format!("{}/uploads/chunked/:session_id/chunk/:chunk_index", API_PREFIX),
            put(handlers::chunked_upload::record_chunk_upload),
        )
        .route(
            &format!("{}/uploads/chunked/:session_id/complete", API_PREFIX),
            post(handlers::chunked_upload::complete_chunked_upload),
        )
        .route(
            &format!("{}/uploads/chunked/:session_id/progress", API_PREFIX),
            get(handlers::chunked_upload::get_chunked_upload_progress),
        )
        .with_state(state)
}
