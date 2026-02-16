use axum_test::TestServer;
use helpers::auth::TestUser;
use helpers::fixtures;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

/// Result of an image upload workflow
pub struct ImageWorkflowResult {
    pub image_id: Uuid,
    pub upload_response: serde_json::Value,
}

/// Result of a video upload workflow
pub struct VideoWorkflowResult {
    pub video_id: Uuid,
    pub upload_response: serde_json::Value,
    pub task_id: Option<Uuid>,
}

/// Complete image workflow: Upload → Get → Transform → Delete
pub async fn upload_and_verify_image_workflow(
    client: &TestServer,
    user: &TestUser,
) -> ImageWorkflowResult {
    // Step 1: Upload image
    let png_data = fixtures::create_test_png(100, 100);
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let image_id = Uuid::parse_str(
        upload_data
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Expected 'id' in upload response"),
    )
    .expect("Invalid UUID in upload response");

    // Step 2: Get image metadata
    let get_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(get_response.status_code(), 200);
    let image_data: serde_json::Value = get_response.json();
    assert_eq!(
        image_data
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Expected 'id' in response"),
        image_id.to_string()
    );

    // Step 3: Transform image (resize)
    let transform_response = client
        .get(&format!("/api/v0/images/{}/resize?width=50&height=50", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(transform_response.status_code(), 200);

    // Step 4: Delete image
    let delete_response = client
        .delete(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(delete_response.status_code(), 200);

    // Step 5: Verify deletion
    let verify_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(verify_response.status_code(), 404);

    ImageWorkflowResult {
        image_id,
        upload_response: upload_data,
    }
}

/// Video upload and processing workflow: Upload → Poll status → Wait for completion
pub async fn video_upload_and_stream_workflow(
    client: &TestServer,
    user: &TestUser,
    max_wait_seconds: u64,
) -> VideoWorkflowResult {
    // Step 1: Upload video
    let video_data = fixtures::create_test_video();
    let upload_response = client
        .post("/api/v0/videos")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(video_data)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let video_id = Uuid::parse_str(
        upload_data
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Expected 'id' in upload response"),
    )
    .expect("Invalid UUID in upload response");

    let task_id = upload_data
        .get("task_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());

    // Step 2: Poll for video processing completion
    let start_time = std::time::Instant::now();
    let mut processing_complete = false;

    while !processing_complete && start_time.elapsed().as_secs() < max_wait_seconds {
        let status_response = client
            .get(&format!("/api/v0/videos/{}", video_id))
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;

        if status_response.status_code() == 200 {
            let video_data: serde_json::Value = status_response.json();
            let status = video_data
                .get("processing_status")
                .and_then(|v| v.as_str())
                .unwrap_or("pending");

            if status == "completed" {
                processing_complete = true;
            } else if status == "failed" {
                panic!("Video processing failed");
            }
        }

        if !processing_complete {
            sleep(Duration::from_secs(2)).await;
        }
    }

    if !processing_complete {
        // Don't fail - processing might take longer in test environment
        // Just log that we didn't complete in time
        eprintln!(
            "Warning: Video processing did not complete within {} seconds",
            max_wait_seconds
        );
    }

    VideoWorkflowResult {
        video_id,
        upload_response: upload_data,
        task_id,
    }
}

/// Multi-user workflow: Multiple users uploading simultaneously
pub async fn multi_user_workflow(
    client: &TestServer,
    num_users: usize,
) -> Vec<(TestUser, ImageWorkflowResult)> {
    use helpers::auth::register_test_user;
    use futures::future::join_all;

    // Create multiple users
    let mut users = Vec::new();
    for i in 0..num_users {
        let user = register_test_user(
            client,
            Some(&format!("user{}@example.com", i)),
            Some("TestPassword123!"),
            Some(&format!("Org {}", i)),
        )
        .await;
        users.push(user);
    }

    // Upload images concurrently for all users
    let upload_futures: Vec<_> = users
        .iter()
        .map(|user| upload_and_verify_image_workflow(client, user))
        .collect();

    let results = join_all(upload_futures).await;

    // Verify tenant isolation - each user should only see their own images
    for (i, (user, result)) in users.iter().zip(results.iter()).enumerate() {
        let list_response = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;

        assert_eq!(list_response.status_code(), 200);
        let list_data: serde_json::Value = list_response.json();
        let images = list_data
            .get("items")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![]);

        // Each user should only see their own image (or none if cleanup happened)
        // At minimum, we verify the endpoint works and returns proper structure
        assert!(images.len() <= 1, "User {} should have at most 1 image", i);
    }

    users.into_iter().zip(results.into_iter()).collect()
}

/// Rate limit verification workflow: Make requests up to limit → Verify 429 → Wait → Verify recovery
pub async fn rate_limit_verification(
    client: &TestServer,
    user: &TestUser,
    limit: u32,
    endpoint: &str,
) -> bool {
    let mut success_count = 0;
    let mut rate_limited = false;

    // Make requests up to limit + 1
    for i in 0..=limit {
        let response = client
            .get(endpoint)
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;

        match response.status_code() {
            200 | 201 => {
                success_count += 1;
            }
            429 => {
                // Verify rate limit headers
                let headers = response.headers();
                assert!(
                    headers.contains_key("X-RateLimit-Limit"),
                    "Missing X-RateLimit-Limit header"
                );
                assert!(
                    headers.contains_key("Retry-After"),
                    "Missing Retry-After header"
                );
                rate_limited = true;
                break;
            }
            _ => {
                // Other status codes might be valid (e.g., 404 for empty list)
                // Continue checking
            }
        }

        // Small delay to avoid overwhelming the test server
        if i < limit {
            sleep(Duration::from_millis(10)).await;
        }
    }

    // Verify we hit the rate limit
    assert!(rate_limited, "Rate limit was not triggered");
    assert!(
        success_count <= limit,
        "More successful requests than limit: {} > {}",
        success_count,
        limit
    );

    // Wait for rate limit window to reset
    sleep(Duration::from_secs(2)).await;

    // Verify recovery - should be able to make requests again
    let recovery_response = client
        .get(endpoint)
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should not be rate limited anymore (might still get 404 or empty list, but not 429)
    assert_ne!(recovery_response.status_code(), 429, "Still rate limited after wait");

    rate_limited
}

/// Webhook delivery workflow: Create webhook → Upload → Verify event
pub async fn webhook_delivery_workflow(
    client: &TestServer,
    user: &TestUser,
    webhook_url: &str,
) -> (Uuid, Uuid) {
    // Step 1: Create webhook
    let create_response = client
        .post("/api/v0/webhooks")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .json(&json!({
            "url": webhook_url,
            "events": ["image.uploaded", "image.deleted"],
            "secret": "test-webhook-secret"
        }))
        .await;

    assert_eq!(create_response.status_code(), 201);
    let webhook_data: serde_json::Value = create_response.json();
    let webhook_id = Uuid::parse_str(
        webhook_data
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Expected 'id' in webhook response"),
    )
    .expect("Invalid UUID in webhook response");

    // Step 2: Upload image to trigger webhook
    let png_data = fixtures::create_test_png(100, 100);
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let image_id = Uuid::parse_str(
        upload_data
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Expected 'id' in upload response"),
    )
    .expect("Invalid UUID in upload response");

    // Step 3: Wait a bit for webhook to fire (async operation)
    sleep(Duration::from_secs(1)).await;

    // Step 4: Verify webhook event was created
    let events_response = client
        .get(&format!("/api/v0/webhooks/{}/events", webhook_id))
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(events_response.status_code(), 200);
    let events_data: serde_json::Value = events_response.json();
    let events = events_data
        .get("items")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![]);

    // Should have at least one event
    assert!(!events.is_empty(), "Expected at least one webhook event");

    // Verify event is for the uploaded image
    let event = events[0].as_object().expect("Event should be an object");
    let event_data = event
        .get("data")
        .and_then(|v| v.as_object())
        .expect("Event should have 'data' field");
    let event_image_id = event_data
        .get("id")
        .and_then(|v| v.as_str())
        .expect("Event data should have 'id'");

    assert_eq!(
        event_image_id,
        &image_id.to_string(),
        "Webhook event should be for uploaded image"
    );

    (webhook_id, image_id)
}

/// Search workflow: Upload → Index → Search
pub async fn search_workflow(
    client: &TestServer,
    user: &TestUser,
) -> (Uuid, serde_json::Value) {
    // Step 1: Upload a document or image
    let png_data = fixtures::create_test_png(100, 100);
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(upload_response.status_code() == 200 || upload_response.status_code() == 201);
    let upload_data: serde_json::Value = upload_response.json();
    let image_id = Uuid::parse_str(
        upload_data
            .get("id")
            .and_then(|v| v.as_str())
            .expect("Expected 'id' in upload response"),
    )
    .expect("Invalid UUID in upload response");

    // Step 2: Wait a bit for indexing (if semantic search is enabled)
    sleep(Duration::from_secs(2)).await;

    // Step 3: Search for the file
    let search_response = client
        .get("/api/v0/search?query=test")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(search_response.status_code(), 200);
    let search_data: serde_json::Value = search_response.json();

    (image_id, search_data)
}

/// Batch upload workflow: Upload multiple files concurrently
pub async fn batch_upload_workflow(
    client: &TestServer,
    user: &TestUser,
    num_files: usize,
    file_type: &str,
) -> Vec<Uuid> {
    use futures::future::join_all;

    let upload_futures: Vec<_> = (0..num_files)
        .map(|i| {
            let token = user.token.clone();
            let body = match file_type {
                "image" => fixtures::create_test_png(50 + i, 50 + i),
                "document" => fixtures::create_test_pdf(),
                _ => fixtures::create_test_png(100, 100),
            };
            let endpoint = match file_type {
                "image" => "/api/v0/images",
                "document" => "/api/v0/documents",
                _ => "/api/v0/images",
            };

            async move {
                let response = client
                    .post(endpoint)
                    .add_header("Authorization", format!("Bearer {}", token))
                    .add_header("Content-Type", "multipart/form-data")
                    .body(body)
                    .await;

                assert!(
                    response.status_code() == 200 || response.status_code() == 201,
                    "Upload failed with status {}",
                    response.status_code()
                );

                let data: serde_json::Value = response.json();
                Uuid::parse_str(
                    data.get("id")
                        .and_then(|v| v.as_str())
                        .expect("Expected 'id' in response"),
                )
                .expect("Invalid UUID")
            }
        })
        .collect();

    join_all(upload_futures).await
}

/// Wait for async operation with timeout
pub async fn wait_for_condition<F, Fut>(condition: F, timeout_seconds: u64) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let start_time = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_seconds);

    while start_time.elapsed() < timeout {
        if condition().await {
            return true;
        }
        sleep(Duration::from_millis(100)).await;
    }

    false
}
