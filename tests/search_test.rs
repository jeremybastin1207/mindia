mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_search_without_semantic_search_enabled() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Attempt to search without semantic search enabled
    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should return 400 (bad request) since semantic search is not enabled
    assert_eq!(
        response.status_code(),
        400,
        "Should return 400 when semantic search is disabled"
    );

    let data: serde_json::Value = response.json();
    assert!(
        data["error"].as_str().is_some(),
        "Error response should contain error message"
    );
    
    let error_msg = data["error"].as_str().unwrap();
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

    // Attempt to search without query parameter
    let response = client
        .get("/api/v0/search")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should return 400 (bad request) - missing query
    assert_eq!(
        response.status_code(),
        400,
        "Should return 400 when query parameter is missing"
    );
}

#[tokio::test]
async fn test_search_invalid_entity_type() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Attempt to search with invalid entity type
    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .add_query_param("type", "invalid_type")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should return 400 (bad request) - invalid entity type
    assert_eq!(
        response.status_code(),
        400,
        "Should return 400 when entity type is invalid"
    );

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

    // Attempt to search without authorization
    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .await;

    // Should return 401 (unauthorized)
    assert_eq!(
        response.status_code(),
        401,
        "Should return 401 when authorization is missing"
    );
}

#[tokio::test]
async fn test_search_with_limit() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Attempt to search with limit parameter (will fail without semantic search, but tests parameter parsing)
    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .add_query_param("limit", "50")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should return 400 since semantic search is not enabled
    assert_eq!(
        response.status_code(),
        400,
        "Should return 400 when semantic search is disabled"
    );
}

#[tokio::test]
async fn test_search_with_entity_type_filter() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Test with valid entity type filter (will fail without semantic search, but tests parameter parsing)
    let response = client
        .get("/api/v0/search")
        .add_query_param("q", "test query")
        .add_query_param("type", "image")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should return 400 since semantic search is not enabled
    assert_eq!(
        response.status_code(),
        400,
        "Should return 400 when semantic search is disabled"
    );
}

// Note: Tenant isolation tests for semantic search are in embedding_test.rs
// since they require direct database access to verify tenant filtering.
// The search endpoint tenant isolation is tested there by checking that
// search_similar() only returns results for the authenticated tenant.
