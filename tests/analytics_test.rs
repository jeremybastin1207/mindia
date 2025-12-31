mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_get_traffic_summary() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/analytics/traffic")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_object());
}

#[tokio::test]
async fn test_get_url_statistics() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/analytics/urls")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_object() || data.is_array());
}

#[tokio::test]
async fn test_get_storage_summary() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/analytics/storage")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_object());
}

#[tokio::test]
async fn test_refresh_storage_metrics() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .post("/api/v0/analytics/storage/refresh")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert!(response.status_code() == 200 || response.status_code() == 202);
}

#[tokio::test]
async fn test_analytics_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let response = client.get("/api/v0/analytics/traffic").await;

    assert_eq!(response.status_code(), 401);
}

