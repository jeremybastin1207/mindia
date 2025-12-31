mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_upload_image() {
    let app = setup_test_app().await;
    let client = app.client();

    // Register user and get token
    let user = register_test_user(client, None, None, None).await;

    // Create a simple test image (1x1 PNG)
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE,
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
        0x08, 0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x18, 0xDD, 0x8D, 0x89, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND chunk
    ];

    // Upload image
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

    let data: serde_json::Value = response.json();
    assert!(data.is_object());
}

#[tokio::test]
async fn test_get_image_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .get(&format!("/api/v0/images/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_delete_image_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .delete(&format!("/api/v0/images/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_upload_image_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let png_data = vec![0x89, 0x50, 0x4E, 0x47];

    let response = client
        .post("/api/v0/images")
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert_eq!(response.status_code(), 401);
}

