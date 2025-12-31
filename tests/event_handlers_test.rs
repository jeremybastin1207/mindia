//! Event handler API tests.
//!
//! Tests the event handler CRUD and discovery endpoints.
//! Requires migrations to be run (event_handlers table).
//! Test helpers must include event handler setup - see tests/helpers/mod.rs.

mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_list_handler_types() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/event-handlers/types")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data["handler_types"].is_array());
    // Webhook executor should always be registered
    let types = data["handler_types"].as_array().unwrap();
    let has_webhook = types
        .iter()
        .any(|t| t["type_name"].as_str() == Some("webhook"));
    assert!(has_webhook, "Webhook executor should be registered");
}

#[tokio::test]
async fn test_list_event_types() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/event-handlers/events")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data["event_types"].is_array());
    let events = data["event_types"].as_array().unwrap();
    assert!(
        events.iter().any(|e| e.as_str() == Some("file.uploaded")),
        "file.uploaded should be in event types"
    );
}

#[tokio::test]
async fn test_create_webhook_event_handler() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .post("/api/v0/event-handlers")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&serde_json::json!({
            "name": "Test Webhook",
            "description": "Sends uploads to external URL",
            "handler_type": "webhook",
            "event_type": "file.uploaded",
            "priority": 10,
            "config": {
                "url": "https://example.com/webhook",
                "signing_secret": "test-secret"
            }
        }))
        .await;

    assert_eq!(response.status_code(), 201);

    let data: serde_json::Value = response.json();
    assert!(data["id"].is_string());
    assert_eq!(data["name"], "Test Webhook");
    assert_eq!(data["handler_type"], "webhook");
    assert_eq!(data["event_type"], "file.uploaded");
}

#[tokio::test]
async fn test_list_event_handlers() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/event-handlers")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_array());
}
