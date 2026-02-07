//! Image API integration tests.
//!
//! Run with: `cargo test -p mindia-api --test images_test`
//! Requires Docker for testcontainers (Postgres).

mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_upload_image() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let png_data = helpers::fixtures::create_minimal_png();
    let response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(response.status_code() == 200 || response.status_code() == 201);
}

#[tokio::test]
async fn test_list_images() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_image_workflow_upload_get_transform_delete() {
    let app = setup_test_app().await;
    let client = app.client();
    let user = register_test_user(client, None, None, None).await;

    let _result = helpers::workflows::upload_and_verify_image_workflow(client, &user).await;
}
