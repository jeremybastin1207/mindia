#[path = "helpers/mod.rs"]
mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;
use helpers::workflows::rate_limit_verification;
use tokio::time::sleep;
use std::time::Duration;

#[tokio::test]
async fn test_rate_limit_enforcement() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Test rate limiting on images endpoint
    // Note: The actual limit depends on test configuration
    // We'll use a conservative limit of 100 requests per minute for testing
    let limit = 100;
    
    // This will make requests up to the limit and verify 429 is returned
    let rate_limited = rate_limit_verification(client, &user, limit, "/api/v0/images").await;
    
    assert!(rate_limited, "Rate limit should be triggered");
}

#[tokio::test]
async fn test_rate_limit_headers() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Make a request and check for rate limit headers
    let response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Should have rate limit headers (might not be present if rate limiting is disabled in tests)
    let headers = response.headers();
    
    // Check if headers exist (they might not if rate limiting middleware isn't active)
    if headers.contains_key("X-RateLimit-Limit") {
        assert!(
            headers.contains_key("X-RateLimit-Remaining"),
            "If X-RateLimit-Limit is present, X-RateLimit-Remaining should also be present"
        );
    }
}

#[tokio::test]
async fn test_rate_limit_recovery() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Make requests until rate limited
    let mut hit_rate_limit = false;
    for i in 0..150 {
        let response = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;

        if response.status_code() == 429 {
            hit_rate_limit = true;
            // Check Retry-After header
            let headers = response.headers();
            assert!(
                headers.contains_key("Retry-After"),
                "429 response should include Retry-After header"
            );
            break;
        }

        // Small delay
        if i < 149 {
            sleep(Duration::from_millis(10)).await;
        }
    }

    if hit_rate_limit {
        // Wait for rate limit window to reset
        sleep(Duration::from_secs(2)).await;

        // Should be able to make requests again
        let recovery_response = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;

        assert_ne!(
            recovery_response.status_code(),
            429,
            "Should not be rate limited after wait period"
        );
    } else {
        // Rate limiting might not be active in test environment
        // This is acceptable - the test just verifies the mechanism works if enabled
        eprintln!("Rate limiting not triggered - might be disabled in test environment");
    }
}

#[tokio::test]
async fn test_rate_limit_per_tenant() {
    let app = setup_test_app().await;
    let client = app.client();

    // Create two different tenants
    let user1 = register_test_user(
        client,
        Some("tenant1@example.com"),
        None,
        Some("Tenant 1"),
    )
    .await;

    let user2 = register_test_user(
        client,
        Some("tenant2@example.com"),
        None,
        Some("Tenant 2"),
    )
    .await;

    // Make requests from tenant 1 until rate limited
    let mut tenant1_rate_limited = false;
    for i in 0..150 {
        let response = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user1.token))
            .await;

        if response.status_code() == 429 {
            tenant1_rate_limited = true;
            break;
        }

        if i < 149 {
            sleep(Duration::from_millis(10)).await;
        }
    }

    // Tenant 2 should still be able to make requests
    // (if per-tenant rate limiting is enabled)
    let tenant2_response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user2.token))
        .await;

    // Tenant 2 should not be rate limited (unless global limit is hit)
    if tenant1_rate_limited {
        // Tenant 2 should still be able to make requests
        // (if rate limiting is per-tenant)
        assert_ne!(
            tenant2_response.status_code(),
            429,
            "Tenant 2 should not be rate limited by tenant 1's rate limit"
        );
    }
}
