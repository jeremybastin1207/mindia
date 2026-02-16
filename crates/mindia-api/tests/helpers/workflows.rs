//! Workflow helpers for integration tests (upload → get → delete, etc.).

#![allow(dead_code)]

use axum_test::multipart::{MultipartForm, Part};
use axum_test::TestServer;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use super::auth::TestUser;
use super::fixtures;

/// Result of an image upload workflow.
pub struct ImageWorkflowResult {
    pub image_id: Uuid,
    pub upload_response: serde_json::Value,
}

/// Upload image, get, transform, delete. Uses `/api/v0/images`.
pub async fn upload_and_verify_image_workflow(
    client: &TestServer,
    user: &TestUser,
) -> ImageWorkflowResult {
    let png_data = fixtures::create_test_png(100, 100);
    let part = Part::bytes(bytes::Bytes::from(png_data))
        .file_name("image.png")
        .mime_type("image/png");
    let multipart = MultipartForm::new().add_part("file", part);
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .multipart(multipart)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let image_id = Uuid::parse_str(
        upload_data
            .get("id")
            .and_then(|v: &serde_json::Value| v.as_str())
            .expect("Expected 'id' in upload response"),
    )
    .expect("Invalid UUID in upload response");

    let get_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(get_response.status_code(), 200);

    let transform_response = client
        .get(&format!(
            "/api/v0/images/{}/resize?width=50&height=50",
            image_id
        ))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(transform_response.status_code(), 200);

    let delete_response = client
        .delete(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(delete_response.status_code(), 200);

    let verify_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(verify_response.status_code(), 404);

    ImageWorkflowResult {
        image_id,
        upload_response: upload_data,
    }
}

/// Wait for a condition with timeout.
pub async fn wait_for_condition<F, Fut>(condition: F, timeout_seconds: u64) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_seconds);
    while start.elapsed() < timeout {
        if condition().await {
            return true;
        }
        sleep(Duration::from_millis(100)).await;
    }
    false
}
