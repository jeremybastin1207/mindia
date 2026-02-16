//! Search API integration tests.
//!
//! Run with: `cargo test -p mindia-api --test search_test`
//! Requires Docker for testcontainers (Postgres).

mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_search_without_semantic_search_enabled() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
    let data: serde_json::Value = response.json();
    let error_msg = data["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("not enabled") || error_msg.contains("required"),
        "Error message should mention semantic search not enabled or query required"
    );
}

#[tokio::test]
async fn test_search_invalid_query() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/search")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
}

#[tokio::test]
async fn test_search_invalid_entity_type() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .add_query_param("type", "invalid_type")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
    let data: serde_json::Value = response.json();
    let error_msg = data["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("Invalid entity type") || error_msg.contains("not enabled"),
        "Error message should mention invalid entity type or semantic search not enabled"
    );
}

#[tokio::test]
async fn test_search_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .await;

    assert_eq!(response.status_code(), 401);
}

#[tokio::test]
async fn test_search_metadata_only_returns_200() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Metadata-only: search_mode=metadata and at least one metadata filter. No semantic search needed.
    let response = client
        .get("/api/v0/search")
        .add_query_param("search_mode", "metadata")
        .add_query_param("metadata.userId", "test-user-1")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);
    let data: serde_json::Value = response.json();
    assert!(data["results"].is_array());
    assert!(data["count"].as_u64().is_some());
}

#[tokio::test]
async fn test_search_metadata_filter_count_limit() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // More than 10 metadata filters (max) should return 400.
    let response = client
        .get("/api/v0/search")
        .add_query_param("search_mode", "metadata")
        .add_query_param("metadata.k1", "v1")
        .add_query_param("metadata.k2", "v2")
        .add_query_param("metadata.k3", "v3")
        .add_query_param("metadata.k4", "v4")
        .add_query_param("metadata.k5", "v5")
        .add_query_param("metadata.k6", "v6")
        .add_query_param("metadata.k7", "v7")
        .add_query_param("metadata.k8", "v8")
        .add_query_param("metadata.k9", "v9")
        .add_query_param("metadata.k10", "v10")
        .add_query_param("metadata.k11", "v11")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
    let data: serde_json::Value = response.json();
    let error_msg = data["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("Too many") || error_msg.contains("metadata filter"),
        "Error should mention filter limit: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_search_invalid_min_similarity() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test")
        .add_query_param("min_similarity", "2.0")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
    let data: serde_json::Value = response.json();
    let error_msg = data["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("min_similarity")
            || error_msg.contains("0.0")
            || error_msg.contains("1.0"),
        "Error should mention min_similarity range: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_search_query_too_long() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let long_q = "a".repeat(16 * 1024 + 1);

    let response = client
        .get("/api/v0/search")
        .add_query_param("q", &long_q)
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
    let data: serde_json::Value = response.json();
    let error_msg = data["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("exceed") || error_msg.contains("characters") || error_msg.contains("q"),
        "Error should mention query length: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_search_metadata_key_reserved() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Reserved prefix _system_ should be rejected.
    let response = client
        .get("/api/v0/search")
        .add_query_param("search_mode", "metadata")
        .add_query_param("metadata._system_x", "value")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 400);
    let data: serde_json::Value = response.json();
    let error_msg = data["error"].as_str().unwrap_or("");
    assert!(
        error_msg.contains("reserved")
            || error_msg.contains("metadata")
            || error_msg.contains("Invalid"),
        "Error should mention reserved key or invalid metadata: {}",
        error_msg
    );
}

#[tokio::test]
async fn test_search_with_limit_and_offset() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Metadata-only with limit/offset (tests parameter parsing; may return empty results).
    let response = client
        .get("/api/v0/search")
        .add_query_param("search_mode", "metadata")
        .add_query_param("metadata.userId", "any")
        .add_query_param("limit", "5")
        .add_query_param("offset", "0")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);
    let data: serde_json::Value = response.json();
    assert!(data["results"].is_array());
    assert!(data["count"].as_u64().is_some());
}
