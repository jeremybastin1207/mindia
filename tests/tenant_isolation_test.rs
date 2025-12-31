mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;
use uuid::Uuid;

/// CRITICAL: Test tenant isolation for images
/// Tenant A uploads an image, Tenant B should NOT be able to access it
#[tokio::test]
async fn test_tenant_isolation_images() {
    let app = setup_test_app().await;
    let client = app.client();
    let pool = app.pool();

    // Register two different tenants
    let tenant_a = register_test_user(client, Some("tenant-a@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b@example.com"), None, None).await;

    // Verify tenants are different
    assert_ne!(
        tenant_a.tenant_id, tenant_b.tenant_id,
        "Tenants should have different IDs"
    );

    // Create a simple test image (1x1 PNG)
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE,
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54,
        0x08, 0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x18, 0xDD, 0x8D, 0x89, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // Tenant A uploads an image
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data.clone())
        .await;

    assert!(
        upload_response.status_code() == 200 || upload_response.status_code() == 201,
        "Tenant A should be able to upload image"
    );

    let upload_data: serde_json::Value = upload_response.json();
    let image_id = upload_data["id"]
        .as_str()
        .expect("Upload response should contain image ID");

    // CRITICAL: Tenant B should NOT be able to access Tenant A's image
    let get_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        get_response.status_code(),
        404,
        "Tenant B should NOT be able to access Tenant A's image (should return 404)"
    );

    // CRITICAL: Tenant B should NOT see Tenant A's image in list
    let list_response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        list_response.status_code(),
        200,
        "List endpoint should return 200"
    );

    let list_data: serde_json::Value = list_response.json();
    let images = if list_data.is_array() {
        &list_data
    } else if list_data["images"].is_array() {
        &list_data["images"]
    } else {
        &list_data
    };

    if let Some(images_array) = images.as_array() {
        // Verify Tenant B's list does NOT contain Tenant A's image
        assert!(
            !images_array.iter().any(|img| img["id"].as_str() == Some(image_id)),
            "Tenant B's image list should NOT contain Tenant A's image"
        );
    }

    // CRITICAL: Tenant B should NOT be able to delete Tenant A's image
    let delete_response = client
        .delete(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        delete_response.status_code(),
        404,
        "Tenant B should NOT be able to delete Tenant A's image (should return 404)"
    );

    // Verify Tenant A can still access their own image
    let get_own_response = client
        .get(&format!("/api/v0/images/{}", image_id))
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .await;

    assert_eq!(
        get_own_response.status_code(),
        200,
        "Tenant A should be able to access their own image"
    );
}

/// CRITICAL: Test tenant isolation for documents
#[tokio::test]
#[cfg(feature = "document")]
async fn test_tenant_isolation_documents() {
    let app = setup_test_app().await;
    let client = app.client();

    // Register two different tenants
    let tenant_a = register_test_user(client, Some("tenant-a-doc@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b-doc@example.com"), None, None).await;

    // Create a simple test PDF (minimal PDF structure)
    let pdf_data = b"%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n>>\nendobj\nxref\n0 1\ntrailer\n<<\n/Size 1\n>>\nstartxref\n9\n%%EOF";

    // Tenant A uploads a document
    let upload_response = client
        .post("/api/v0/documents")
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(pdf_data.to_vec())
        .await;

    assert!(
        upload_response.status_code() == 200 || upload_response.status_code() == 201,
        "Tenant A should be able to upload document"
    );

    let upload_data: serde_json::Value = upload_response.json();
    let document_id = upload_data["id"]
        .as_str()
        .expect("Upload response should contain document ID");

    // CRITICAL: Tenant B should NOT be able to access Tenant A's document
    let get_response = client
        .get(&format!("/api/v0/documents/{}", document_id))
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        get_response.status_code(),
        404,
        "Tenant B should NOT be able to access Tenant A's document (should return 404)"
    );
}

/// CRITICAL: Test tenant isolation for videos
#[tokio::test]
#[cfg(feature = "video")]
async fn test_tenant_isolation_videos() {
    let app = setup_test_app().await;
    let client = app.client();

    // Register two different tenants
    let tenant_a = register_test_user(client, Some("tenant-a-vid@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b-vid@example.com"), None, None).await;

    // Create a minimal test video (just headers - actual video file would be large)
    // For testing, we'll use a very small fake MP4 header
    let video_data = b"ftypmp41\x00\x00\x00\x20mdat";

    // Tenant A uploads a video (this might fail validation, but tests the endpoint)
    let upload_response = client
        .post("/api/v0/videos")
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(video_data.to_vec())
        .await;

    // If upload succeeds, test isolation
    if upload_response.status_code() == 200 || upload_response.status_code() == 201 {
        let upload_data: serde_json::Value = upload_response.json();
        if let Some(video_id_str) = upload_data["id"].as_str() {
            // CRITICAL: Tenant B should NOT be able to access Tenant A's video
            let get_response = client
                .get(&format!("/api/v0/videos/{}", video_id_str))
                .add_header("Authorization", format!("Bearer {}", tenant_b.token))
                .await;

            assert_eq!(
                get_response.status_code(),
                404,
                "Tenant B should NOT be able to access Tenant A's video (should return 404)"
            );
        }
    }
}

/// CRITICAL: Test tenant isolation for API keys
#[tokio::test]
async fn test_tenant_isolation_api_keys() {
    let app = setup_test_app().await;
    let client = app.client();

    // Register two different tenants
    let tenant_a = register_test_user(client, Some("tenant-a-keys@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b-keys@example.com"), None, None).await;

    // Tenant A creates an API key
    let create_response = client
        .post("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .json(&serde_json::json!({
            "name": "Tenant A Key",
            "expires_at": null
        }))
        .await;

    assert_eq!(
        create_response.status_code(),
        201,
        "Tenant A should be able to create API key"
    );

    let create_data: serde_json::Value = create_response.json();
    let key_id = create_data["id"]
        .as_str()
        .expect("Create response should contain API key ID");

    // CRITICAL: Tenant B should NOT be able to access Tenant A's API key
    let get_response = client
        .get(&format!("/api/v0/api-keys/{}", key_id))
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        get_response.status_code(),
        404,
        "Tenant B should NOT be able to access Tenant A's API key (should return 404)"
    );

    // CRITICAL: Tenant B should NOT see Tenant A's API key in list
    let list_response = client
        .get("/api/v0/api-keys")
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        list_response.status_code(),
        200,
        "List endpoint should return 200"
    );

    let list_data: serde_json::Value = list_response.json();
    let keys = if list_data.is_array() {
        &list_data
    } else if list_data["keys"].is_array() {
        &list_data["keys"]
    } else {
        &list_data
    };

    if let Some(keys_array) = keys.as_array() {
        // Verify Tenant B's list does NOT contain Tenant A's key
        assert!(
            !keys_array.iter().any(|key| key["id"].as_str() == Some(key_id)),
            "Tenant B's API key list should NOT contain Tenant A's key"
        );
    }
}

/// CRITICAL: Test tenant isolation for webhooks
#[tokio::test]
async fn test_tenant_isolation_webhooks() {
    let app = setup_test_app().await;
    let client = app.client();

    // Register two different tenants
    let tenant_a = register_test_user(client, Some("tenant-a-web@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b-web@example.com"), None, None).await;

    // Tenant A creates a webhook
    let create_response = client
        .post("/api/v0/webhooks")
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .json(&serde_json::json!({
            "url": "https://tenant-a.example.com/webhook",
            "events": ["image.uploaded"],
            "secret": "tenant-a-secret"
        }))
        .await;

    assert_eq!(
        create_response.status_code(),
        201,
        "Tenant A should be able to create webhook"
    );

    let create_data: serde_json::Value = create_response.json();
    let webhook_id = create_data["id"]
        .as_str()
        .expect("Create response should contain webhook ID");

    // CRITICAL: Tenant B should NOT be able to access Tenant A's webhook
    let get_response = client
        .get(&format!("/api/v0/webhooks/{}", webhook_id))
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    assert_eq!(
        get_response.status_code(),
        404,
        "Tenant B should NOT be able to access Tenant A's webhook (should return 404)"
    );
}

/// CRITICAL: Test cross-tenant enumeration attack prevention
/// Attempts to enumerate other tenants' IDs should fail
#[tokio::test]
async fn test_cross_tenant_enumeration_attack() {
    let app = setup_test_app().await;
    let client = app.client();

    // Register two tenants
    let tenant_a = register_test_user(client, Some("tenant-a-enum@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b-enum@example.com"), None, None).await;

    // Create a simple test image
    let png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE,
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54,
        0x08, 0xD7, 0x63, 0xF8, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
        0x00, 0x18, 0xDD, 0x8D, 0x89, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // Tenant A uploads an image
    let upload_response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", tenant_a.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    assert!(
        upload_response.status_code() == 200 || upload_response.status_code() == 201,
        "Tenant A should be able to upload image"
    );

    let upload_data: serde_json::Value = upload_response.json();
    let image_id_str = upload_data["id"]
        .as_str()
        .expect("Upload response should contain image ID");

    // CRITICAL: Tenant B attempts to access Tenant A's image using known ID
    // This simulates an enumeration attack where attacker tries various IDs
    let get_response = client
        .get(&format!("/api/v0/images/{}", image_id_str))
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    // Should return 404, NOT 403 or 401, to prevent information disclosure
    // Returning 404 hides whether the resource exists or not
    assert_eq!(
        get_response.status_code(),
        404,
        "Should return 404 (not found) to prevent enumeration, not 403 (forbidden)"
    );

    // CRITICAL: Tenant B attempts to access random UUIDs - should all return 404
    let random_ids = vec![
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    ];

    for random_id in random_ids {
        let enum_response = client
            .get(&format!("/api/v0/images/{}", random_id))
            .add_header("Authorization", format!("Bearer {}", tenant_b.token))
            .await;

        // Should return 404 consistently - doesn't reveal whether resource exists
        assert_eq!(
            enum_response.status_code(),
            404,
            "Enumeration attempt should return 404, not reveal resource existence"
        );
    }
}
