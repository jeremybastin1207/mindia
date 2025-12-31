#[path = "helpers/mod.rs"]
mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;
use uuid::Uuid;

#[tokio::test]
async fn test_invalid_authentication() {
    let app = setup_test_app().await;
    let client = app.client();

    // Try to access protected endpoint without auth
    let response = client.get("/api/v0/images").await;

    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_invalid_token() {
    let app = setup_test_app().await;
    let client = app.client();

    // Try to access with invalid token
    let response = client
        .get("/api/v0/images")
        .add_header("Authorization", "Bearer invalid-token-here")
        .await;

    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_invalid_file_type() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Try to upload invalid file type
    let invalid_data = b"not an image file";
    let response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(invalid_data.to_vec())
        .await;

    // Should reject invalid file type (400 or 415)
    assert!(
        response.status_code() >= 400 && response.status_code() < 500,
        "Should reject invalid file type"
    );
}

#[tokio::test]
async fn test_file_too_large() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Create a file larger than the limit (assuming 10MB limit from test config)
    use helpers::fixtures::create_file_of_size;
    let large_file = create_file_of_size(11 * 1024 * 1024, "image"); // 11MB

    let response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(large_file)
        .await;

    // Should reject file that's too large (413 or 400)
    assert!(
        response.status_code() == 413 || response.status_code() == 400,
        "Should reject file that's too large, got status {}",
        response.status_code()
    );
}

#[tokio::test]
async fn test_missing_file() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Try to access non-existent file
    let fake_id = Uuid::new_v4();
    let response = client
        .get(&format!("/api/v0/images/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_delete_missing_file() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Try to delete non-existent file
    let fake_id = Uuid::new_v4();
    let response = client
        .delete(&format!("/api/v0/images/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_invalid_uuid_format() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Try to access with invalid UUID format
    let response = client
        .get("/api/v0/images/not-a-valid-uuid")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should return 400 Bad Request for invalid UUID
    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_cross_tenant_access() {
    let app = setup_test_app().await;
    let client = app.client();

    // Create two different tenants
    let user1 = register_test_user(
        client,
        Some("tenant1@example.com"),
        None,
        Some("Tenant 1"),
    )
    .await;

    let user2 = register_test_user(
        client,
        Some("tenant2@example.com"),
        None,
        Some("Tenant 2"),
    )
    .await;

    // User 1 uploads a file
    use helpers::fixtures::create_minimal_png;
    let png_data = create_minimal_png();
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user1.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let image_id = upload_data
        .get("id")
        .and_then(|v| v.as_str())
        .expect("Expected 'id' in response");

    // User 2 tries to access user 1's file
    let access_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user2.token))
        .await;

    // Should be denied access (404 or 403)
    assert!(
        access_response.status_code() == 404 || access_response.status_code() == 403,
        "Cross-tenant access should be denied, got status {}",
        access_response.status_code()
    );
}

#[tokio::test]
async fn test_malformed_json() {
    let app = setup_test_app().await;
    let client = app.client();

    // Try to send malformed JSON in request body
    let response = client
        .post("/api/v0/auth/login")
        .add_header("Content-Type", "application/json")
        .body(b"{ invalid json }")
        .await;

    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_missing_required_fields() {
    let app = setup_test_app().await;
    let client = app.client();

    // Try to register without required fields
    let response = client
        .post("/api/v0/auth/register")
        .json(&serde_json::json!({
            "organization_name": "Test"
            // Missing required fields
        }))
        .await;

    assert_eq!(response.status_code(), 400);
}
