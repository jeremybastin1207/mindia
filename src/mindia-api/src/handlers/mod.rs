pub mod analytics;
#[cfg(feature = "audio")]
pub mod audio_download;
#[cfg(feature = "audio")]
pub mod audio_get;
#[cfg(feature = "audio")]
pub mod audio_upload;
pub mod batch;
pub mod chunked_upload;
pub mod config;
#[cfg(feature = "document")]
pub mod document_download;
#[cfg(feature = "document")]
pub mod document_get;
#[cfg(feature = "document")]
pub mod document_upload;
pub mod file_group;
pub mod folders;
pub mod image_download;
pub mod image_get;
pub mod image_upload;
pub mod image_upload_url;
pub mod media_delete;
pub mod media_get;
pub mod metadata;
pub mod named_transformations;
#[cfg(feature = "plugin")]
pub mod plugins;
pub mod presigned_upload;
pub mod search;
pub mod tasks;
pub mod transform;
#[cfg(feature = "video")]
pub mod video_get;
#[cfg(feature = "video")]
pub mod video_stream;
#[cfg(feature = "video")]
pub mod video_upload;
pub mod webhooks;

#[allow(dead_code)]
fn _assert_multipart_send() {
    use axum::extract::Multipart;

    fn assert_send<T: Send>() {}
    assert_send::<Multipart>();
}
