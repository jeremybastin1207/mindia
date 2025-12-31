#[cfg(feature = "video")]
mod helpers;

#[cfg(feature = "video")]
use helpers::auth::register_test_user;
#[cfg(feature = "video")]
use helpers::setup_test_app;

#[cfg(feature = "video")]
#[tokio::test]
async fn test_list_videos() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    let response = client
        .get("/api/v0/videos")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 200);

    let data: serde_json::Value = response.json();
    assert!(data.is_object() || data.is_array());
}

#[tokio::test]
async fn test_get_video_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .get(&format!("/api/v0/videos/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_delete_video_not_found() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;
    let fake_id = uuid::Uuid::new_v4();

    let response = client
        .delete(&format!("/api/v0/videos/{}", fake_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(response.status_code(), 404);
}

#[tokio::test]
async fn test_videos_unauthorized() {
    let app = setup_test_app().await;
    let client = app.client();

    let response = client.get("/api/v0/videos").await;

    assert_eq!(response.status_code(), 401);
}

