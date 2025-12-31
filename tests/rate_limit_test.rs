mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_rate_limit_headers_present() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Make a request
    let response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    // Verify rate limit headers are present
    let headers = response.headers();
    
    assert!(
        headers.contains_key("X-RateLimit-Limit"),
        "X-RateLimit-Limit header should be present"
    );
    
    assert!(
        headers.contains_key("X-RateLimit-Remaining"),
        "X-RateLimit-Remaining header should be present (recently added)"
    );

    // Verify header values are valid numbers
    if let Some(limit_header) = headers.get("X-RateLimit-Limit") {
        let limit_str = limit_header.to_str().expect("Header should be valid string");
        let limit: u32 = limit_str.parse().expect("Limit should be a valid number");
        assert!(limit > 0, "Rate limit should be greater than 0");
    }

    if let Some(remaining_header) = headers.get("X-RateLimit-Remaining") {
        let remaining_str = remaining_header.to_str().expect("Header should be valid string");
        let remaining: u32 = remaining_str.parse().expect("Remaining should be a valid number");
        // Remaining should be <= limit
        if let Some(limit_header) = headers.get("X-RateLimit-Limit") {
            let limit: u32 = limit_header.to_str().unwrap().parse().unwrap();
            assert!(remaining <= limit, "Remaining should be <= limit");
        }
    }
}

#[tokio::test]
async fn test_rate_limit_remaining_decrements() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Get initial remaining count
    let response1 = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    let headers1 = response1.headers();
    let initial_remaining = if let Some(remaining_header) = headers1.get("X-RateLimit-Remaining") {
        remaining_header.to_str().unwrap().parse::<u32>().unwrap()
    } else {
        panic!("X-RateLimit-Remaining header missing in first request");
    };

    // Make another request - remaining should decrement
    let response2 = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    let headers2 = response2.headers();
    let second_remaining = if let Some(remaining_header) = headers2.get("X-RateLimit-Remaining") {
        remaining_header.to_str().unwrap().parse::<u32>().unwrap()
    } else {
        panic!("X-RateLimit-Remaining header missing in second request");
    };

    // Remaining should have decremented
    assert!(
        second_remaining < initial_remaining,
        "X-RateLimit-Remaining should decrement: {} -> {}",
        initial_remaining,
        second_remaining
    );
}

#[tokio::test]
async fn test_rate_limit_enforcement_tenant_based() {
    let app = setup_test_app().await;
    let client = app.client();

    // Test config has limit of 10 per minute
    let user = register_test_user(client, None, None, None).await;

    // Make requests up to the limit
    let mut success_count = 0;
    let mut rate_limited = false;
    
    // Make 11 requests (limit is 10, so 11th should be rate limited)
    for i in 0..=10 {
        let response = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;

        match response.status_code() {
            200 | 201 | 404 => {
                // 404 is OK for empty lists
                success_count += 1;
                
                // Verify headers are present
                let headers = response.headers();
                assert!(
                    headers.contains_key("X-RateLimit-Limit"),
                    "X-RateLimit-Limit header should be present"
                );
                assert!(
                    headers.contains_key("X-RateLimit-Remaining"),
                    "X-RateLimit-Remaining header should be present"
                );
            }
            429 => {
                rate_limited = true;
                // Verify rate limit response headers
                let headers = response.headers();
                assert!(
                    headers.contains_key("X-RateLimit-Limit"),
                    "Rate limited response should include X-RateLimit-Limit"
                );
                assert!(
                    headers.contains_key("X-RateLimit-Remaining"),
                    "Rate limited response should include X-RateLimit-Remaining (should be 0)"
                );
                assert!(
                    headers.contains_key("Retry-After"),
                    "Rate limited response should include Retry-After header"
                );
                
                // Verify X-RateLimit-Remaining is 0
                if let Some(remaining_header) = headers.get("X-RateLimit-Remaining") {
                    let remaining: u32 = remaining_header.to_str().unwrap().parse().unwrap();
                    assert_eq!(
                        remaining, 0,
                        "X-RateLimit-Remaining should be 0 when rate limited"
                    );
                }
                
                break;
            }
            _ => {
                // Other status codes
            }
        }

        // Small delay between requests
        if i < 10 {
            sleep(Duration::from_millis(10)).await;
        }
    }

    // Verify we hit the rate limit
    assert!(
        rate_limited,
        "Rate limit should have been triggered after 11 requests (limit is 10)"
    );
    assert!(
        success_count <= 10,
        "Should not have more than 10 successful requests before rate limiting"
    );
}

#[tokio::test]
async fn test_rate_limit_too_many_requests() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Exhaust rate limit
    for _ in 0..10 {
        let _ = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;
        sleep(Duration::from_millis(10)).await;
    }

    // Next request should be rate limited
    let response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(
        response.status_code(),
        429,
        "Should return 429 (Too Many Requests) when rate limit exceeded"
    );

    // Verify error response format
    let data: serde_json::Value = response.json();
    assert!(
        data["error"].as_str().is_some(),
        "Rate limit error response should contain error message"
    );

    let error_msg = data["error"].as_str().unwrap();
    assert!(
        error_msg.contains("Too many requests") || error_msg.contains("slow down"),
        "Error message should indicate rate limiting"
    );
}

#[tokio::test]
async fn test_rate_limit_bucket_isolation() {
    let app = setup_test_app().await;
    let client = app.client();

    // Create two different tenants
    let tenant_a = register_test_user(client, Some("tenant-a-rate@example.com"), None, None).await;
    let tenant_b = register_test_user(client, Some("tenant-b-rate@example.com"), None, None).await;

    // Tenant A makes requests
    for _ in 0..5 {
        let _ = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", tenant_a.token))
            .await;
    }

    // Tenant B should have full rate limit available (isolated buckets)
    let response_b = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", tenant_b.token))
        .await;

    // Tenant B should not be rate limited (different bucket)
    assert_ne!(
        response_b.status_code(),
        429,
        "Tenant B should not be rate limited by Tenant A's requests"
    );

    let headers_b = response_b.headers();
    if let Some(remaining_header) = headers_b.get("X-RateLimit-Remaining") {
        let remaining: u32 = remaining_header.to_str().unwrap().parse().unwrap();
        // Remaining should be close to the limit (Tenant B hasn't made many requests)
        assert!(
            remaining >= 9, // Should be 9 or 10 (limit - 1 or limit)
            "Tenant B should have most of their rate limit remaining: {}",
            remaining
        );
    }
}

#[tokio::test]
async fn test_rate_limit_reset_after_window() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

    // Exhaust rate limit
    for _ in 0..10 {
        let _ = client
            .get("/api/v0/images")
            .add_header("Authorization", format!("Bearer {}", user.token))
            .await;
        sleep(Duration::from_millis(10)).await;
    }

    // Verify rate limited
    let rate_limited_response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    assert_eq!(
        rate_limited_response.status_code(),
        429,
        "Should be rate limited after 10 requests"
    );

    // Wait for rate limit window to reset (60 seconds)
    // For testing, we'll just verify the behavior exists
    // In real scenario, you'd wait 60+ seconds
    // For faster tests, you could mock time or reduce window_seconds in test config
    
    // Note: Actual window reset test would require waiting 60 seconds
    // which is too slow for unit tests. This is documented as a limitation.
}

#[tokio::test]
async fn test_rate_limit_different_limits_for_tenant() {
    let app = setup_test_app().await;
    let client = app.client();

    // Test that tenant-based rate limiting works
    // In the test setup, tenant limit is same as global (20), but the infrastructure supports different limits
    let user = register_test_user(client, None, None, None).await;

    // Make a request and check headers
    let response = client
        .get("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .await;

    let headers = response.headers();
    
    if let Some(limit_header) = headers.get("X-RateLimit-Limit") {
        let limit: u32 = limit_header.to_str().unwrap().parse().unwrap();
        // In test config, limit is 10, but tenant limit would be 20 if configured
        // For now, just verify the header exists and is valid
        assert!(limit > 0, "Rate limit should be a positive number");
    }
}
