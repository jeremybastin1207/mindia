//! Workflow API integration tests.
//!
//! Run with: `cargo test -p mindia-api --test workflow_test --features workflow`
//! Requires Docker for testcontainers (Postgres).

#![cfg(feature = "workflow")]

mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;
use serde_json::json;

#[tokio::test]
async fn test_create_list_get_update_delete_workflow() {
    let app = helpers::setup_test_app().await;
    let client = app.client();
    let user = register_test_user(client, None, None, None).await;

    let create_body = json!({
        "name": "Test workflow",
        "description": "Description",
        "enabled": true,
        "steps": [
            { "action": "plugin", "plugin_name": "openai_image_description" }
        ],
        "trigger_on_upload": false,
        "stop_on_failure": true
    });

    let create_res = client
        .post("/api/v0/workflows")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&create_body)
        .await;
    assert_eq!(create_res.status_code(), 200, "create workflow");
    let created: serde_json::Value = create_res.json();
    let workflow_id = created
        .get("id")
        .and_then(|v| v.as_str())
        .expect("id in response");
    assert_eq!(
        created.get("name").and_then(|v| v.as_str()),
        Some("Test workflow")
    );

    let list_res = client
        .get("/api/v0/workflows")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(list_res.status_code(), 200, "list workflows");
    let list: Vec<serde_json::Value> = list_res.json();
    assert!(!list.is_empty());
    assert!(list
        .iter()
        .any(|w| w.get("id").and_then(|v| v.as_str()) == Some(workflow_id)));

    let get_res = client
        .get(&format!("/api/v0/workflows/{}", workflow_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(get_res.status_code(), 200, "get workflow");
    let got: serde_json::Value = get_res.json();
    assert_eq!(
        got.get("name").and_then(|v| v.as_str()),
        Some("Test workflow")
    );

    let update_body = json!({
        "name": "Updated workflow name",
        "enabled": false
    });
    let update_res = client
        .put(&format!("/api/v0/workflows/{}", workflow_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&update_body)
        .await;
    assert_eq!(update_res.status_code(), 200, "update workflow");
    let updated: serde_json::Value = update_res.json();
    assert_eq!(
        updated.get("name").and_then(|v| v.as_str()),
        Some("Updated workflow name")
    );
    assert_eq!(updated.get("enabled"), Some(&json!(false)));

    let delete_res = client
        .delete(&format!("/api/v0/workflows/{}", workflow_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(delete_res.status_code(), 204, "delete workflow");

    let get_after = client
        .get(&format!("/api/v0/workflows/{}", workflow_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;
    assert_eq!(get_after.status_code(), 404, "workflow should be gone");
}

#[tokio::test]
async fn test_workflow_validation_empty_name() {
    let app = helpers::setup_test_app().await;
    let client = app.client();
    let user = register_test_user(client, None, None, None).await;

    let body = json!({
        "name": "   ",
        "steps": [{ "action": "plugin", "plugin_name": "openai_image_description" }]
    });
    let res = client
        .post("/api/v0/workflows")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&body)
        .await;
    assert_eq!(res.status_code(), 400);
}

#[tokio::test]
async fn test_workflow_validation_invalid_steps() {
    let app = helpers::setup_test_app().await;
    let client = app.client();
    let user = register_test_user(client, None, None, None).await;

    let body = json!({
        "name": "Bad steps",
        "steps": "not an array"
    });
    let res = client
        .post("/api/v0/workflows")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&body)
        .await;
    assert_eq!(res.status_code(), 400);
}
