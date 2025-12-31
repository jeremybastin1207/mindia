#[cfg(feature = "audio")]
mod helpers;

#[cfg(feature = "audio")]
use helpers::auth::register_test_user;
#[cfg(feature = "audio")]
use helpers::setup_test_app;

#[cfg(feature = "audio")]
#[tokio::test]
async fn test_list_audios() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/audios")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_object() || data.is_array());
}

#[tokio::test]
async fn test_get_audio_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .get(&format!("/api/v0/audios/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_delete_audio_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .delete(&format!("/api/v0/audios/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_audios_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let response = client.get("/api/v0/audios").await;

    assert_eq!(response.status_code(), 401);
}

