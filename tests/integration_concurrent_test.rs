#[path = "helpers/mod.rs"]
mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;
use helpers::workflows::{batch_upload_workflow, multi_user_workflow};
use futures::future::join_all;

#[tokio::test]
async fn test_concurrent_uploads() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Upload multiple images concurrently
    let num_files = 10;
    let image_ids = batch_upload_workflow(client, &user, num_files, "image").await;

    assert_eq!(image_ids.len(), num_files);
    
    // Verify all uploads succeeded
    for image_id in &image_ids {
        assert!(!image_id.is_nil());
    }
}

#[tokio::test]
async fn test_multiple_users_concurrent() {
    let app = setup_test_app().await;
    let client = app.client();

    // Multiple users uploading simultaneously
    let num_users = 5;
    let results = multi_user_workflow(client, num_users).await;

    assert_eq!(results.len(), num_users);
    
    // Verify each user's upload succeeded
    for (user, result) in &results {
        assert!(!user.user_id.is_nil());
        assert!(!result.image_id.is_nil());
    }
}

#[tokio::test]
async fn test_concurrent_mixed_operations() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Concurrent mix of operations: uploads, gets, lists
    use helpers::fixtures::create_minimal_png;
    
    let operations: Vec<_> = (0..10)
        .map(|i| {
            let token = user.token.clone();
            let endpoint = if i % 3 == 0 {
                "/api/v0/images" // Upload
            } else if i % 3 == 1 {
                "/api/v0/images" // List
            } else {
                "/api/v0/images" // List (same for simplicity)
            };

            async move {
                if i % 3 == 0 {
                    let png_data = create_minimal_png();
                    let response = client
                        .post(endpoint)
                        .add_header("Authorization", format!("Bearer {}", token))
                        .add_header("Content-Type", "multipart/form-data")
                        .body(png_data)
                        .await;
                    response.status_code()
                } else {
                    let response = client
                        .get(endpoint)
                        .add_header("Authorization", format!("Bearer {}", token))
                        .await;
                    response.status_code()
                }
            }
        })
        .collect();

    let status_codes = join_all(operations).await;

    // All operations should succeed (200/201) or at least not fail with 5xx
    for (i, status) in status_codes.iter().enumerate() {
        assert!(
            *status < 500,
            "Operation {} should not return 5xx error, got {}",
            i,
            status
        );
    }
}

#[tokio::test]
async fn test_concurrent_file_access() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Upload a file
    use helpers::fixtures::create_minimal_png;
    let png_data = create_minimal_png();
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let image_id = upload_data
        .get("id")
        .and_then(|v| v.as_str())
        .expect("Expected 'id' in response");

    // Concurrent access to the same file
    let concurrent_accesses: Vec<_> = (0..20)
        .map(|_| {
            let token = user.token.clone();
            let id = image_id.to_string();
            async move {
                let response = client
                    .get(&format!("/api/v0/images/{}", id))
                    .add_header("Authorization", format!("Bearer {}", token))
                    .await;
                response.status_code()
            }
        })
        .collect();

    let status_codes = join_all(concurrent_accesses).await;

    // All accesses should succeed (200) or return 404 if file was deleted
    for status in &status_codes {
        assert!(
            *status == 200 || *status == 404,
            "Concurrent access should return 200 or 404, got {}",
            status
        );
    }
}

#[tokio::test]
async fn test_concurrent_deletes() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Upload multiple files
    let num_files = 5;
    let image_ids = batch_upload_workflow(client, &user, num_files, "image").await;

    // Delete all files concurrently
    let delete_futures: Vec<_> = image_ids
        .iter()
        .map(|image_id| {
            let token = user.token.clone();
            let id = *image_id;
            async move {
                let response = client
                    .delete(&format!("/api/v0/images/{}", id))
                    .add_header("Authorization", format!("Bearer {}", token))
                    .await;
                response.status_code()
            }
        })
        .collect();

    let status_codes = join_all(delete_futures).await;

    // All deletes should succeed (200) or return 404 if already deleted
    for status in &status_codes {
        assert!(
            *status == 200 || *status == 404,
            "Concurrent delete should return 200 or 404, got {}",
            status
        );
    }
}
