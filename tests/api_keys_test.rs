mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_create_api_key() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .post("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&serde_json::json!({
            "name": "Test API Key"
        }))
        .await;

    assert_eq!(response.status_code(), 201);

    let data: serde_json::Value = response.json();
    assert!(data["api_key"].as_str().is_some());
    assert!(data["api_key"].as_str().unwrap().starts_with("mk_live_"));
}

#[tokio::test]
async fn test_list_api_keys() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Create an API key first
    let create_response = client
        .post("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&serde_json::json!({
            "name": "Test Key"
        }))
        .await;

    assert_eq!(create_response.status_code(), 201);

    // List API keys
    let response = client
        .get("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_array() || (data.is_object() && data["keys"].is_array()));
}

#[tokio::test]
async fn test_get_api_key() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Create an API key
    let create_response = client
        .post("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&serde_json::json!({
            "name": "Test Key"
        }))
        .await;

    assert_eq!(create_response.status_code(), 201);
    let create_data: serde_json::Value = create_response.json();
    let key_id = create_data["id"].as_str().expect("API key ID should be present");

    // Get the API key
    let response = client
        .get(&format!("/api/v0/api-keys/{}", key_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert_eq!(data["id"], serde_json::json!(key_id));
}

#[tokio::test]
async fn test_revoke_api_key() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Create an API key
    let create_response = client
        .post("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&serde_json::json!({
            "name": "Test Key"
        }))
        .await;

    assert_eq!(create_response.status_code(), 201);
    let create_data: serde_json::Value = create_response.json();
    let key_id = create_data["id"].as_str().expect("API key ID should be present");

    // Revoke the API key
    let response = client
        .delete(&format!("/api/v0/api-keys/{}", key_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_api_keys_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let response = client.get("/api/v0/api-keys").await;

    assert_eq!(response.status_code(), 401);
}

