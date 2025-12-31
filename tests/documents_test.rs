#[cfg(feature = "document")]
mod helpers;

#[cfg(feature = "document")]
use helpers::auth::register_test_user;
#[cfg(feature = "document")]
use helpers::setup_test_app;

#[cfg(feature = "document")]
#[tokio::test]
async fn test_list_documents() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/documents")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_object() || data.is_array());
}

#[tokio::test]
async fn test_get_document_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .get(&format!("/api/v0/documents/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_delete_document_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .delete(&format!("/api/v0/documents/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_documents_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let response = client.get("/api/v0/documents").await;

    assert_eq!(response.status_code(), 401);
}

