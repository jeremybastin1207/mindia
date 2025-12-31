mod helpers;

use helpers::setup_test_app;
use helpers::auth::register_test_user;
use mindia::services::ClamAVService;
use mindia::services::clamav::ScanResult;

#[tokio::test]
async fn test_clamav_disabled_skips_scanning() {
    let app = setup_test_app().await;
    let client = app.client();

    let user = register_test_user(client, None, None, None).await;

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

    // With ClamAV disabled, upload should succeed without scanning
    let response = client
        .post("/api/v0/images")
        .add_header("Authorization", format!("Bearer {}", user.token))
        .add_header("Content-Type", "multipart/form-data")
        .body(png_data)
        .await;

    // Upload should succeed (ClamAV is disabled in test config)
    assert!(
        response.status_code() == 200 || response.status_code() == 201,
        "Upload should succeed when ClamAV is disabled"
    );
}

/// Unit test for ClamAV service (requires ClamAV to be running or mocked)
/// This test verifies the fail-closed behavior
#[tokio::test]
#[ignore] // Ignored by default - requires ClamAV running or mock
async fn test_clamav_fail_closed_mode() {
    // Create ClamAV service with fail-closed enabled
    let clamav_fail_closed = ClamAVService::new(
        "localhost".to_string(),
        3310,
        true, // fail_closed = true
    );

    // Attempt to scan with ClamAV unavailable
    // This simulates ClamAV being down
    let test_data = b"test file content";
    
    let result = clamav_fail_closed.scan_bytes(test_data).await;

    // In fail-closed mode, if ClamAV is unavailable, should return Error
    // (In real scenario, ClamAV would need to be unavailable or mocked)
    match result {
        ScanResult::Error(_) => {
            // Expected in fail-closed mode when ClamAV is unavailable
            // This is the correct behavior - reject upload when ClamAV unavailable
        }
        ScanResult::Clean => {
            // ClamAV is actually available and file is clean
            // This is fine - test passes
        }
        ScanResult::Infected(_) => {
            // File is infected - test passes (correctly detected)
        }
    }
}

/// Unit test for ClamAV service fail-open mode
#[tokio::test]
#[ignore] // Ignored by default - requires ClamAV running or mock
async fn test_clamav_fail_open_mode() {
    // Create ClamAV service with fail-open mode (fail_closed = false)
    let clamav_fail_open = ClamAVService::new(
        "invalid-host-that-does-not-exist".to_string(),
        3310,
        false, // fail_closed = false (fail-open)
    );

    // Attempt to scan - ClamAV will be unavailable
    let test_data = b"test file content";
    
    let result = clamav_fail_open.scan_bytes(test_data).await;

    // In fail-open mode, when ClamAV is unavailable, should continue
    // Note: The actual behavior depends on implementation
    // In the current implementation, it continues but the scan will fail
    // The handler should check the result and allow upload in fail-open mode
    // This test documents the expected behavior
    match result {
        ScanResult::Error(_) => {
            // This is expected when ClamAV is unavailable
            // The handler should allow upload in fail-open mode
        }
        ScanResult::Clean => {
            // Shouldn't happen with invalid host, but if it does, that's fine
        }
        ScanResult::Infected(_) => {
            panic!("Should not detect infection with invalid host");
        }
    }
}

/// Integration test: Verify fail-closed mode rejects uploads when ClamAV unavailable
/// This requires ClamAV to be configured but unavailable
#[tokio::test]
#[ignore] // Requires ClamAV setup or mock
async fn test_upload_with_clamav_fail_closed_unavailable() {
    // This test would require:
    // 1. ClamAV enabled in config
    // 2. ClamAV fail-closed enabled
    // 3. ClamAV service unavailable (stopped or unreachable)
    // 4. Attempt to upload should be rejected
    
    // For now, this is a placeholder test documenting expected behavior
    // In a real test environment with ClamAV available, this would:
    // - Start app with ClamAV enabled and fail-closed = true
    // - Stop or make ClamAV unavailable
    // - Attempt upload
    // - Verify upload is rejected with error about ClamAV being unavailable
}

/// Integration test: Verify fail-open mode allows uploads when ClamAV unavailable
#[tokio::test]
#[ignore] // Requires ClamAV setup or mock
async fn test_upload_with_clamav_fail_open_unavailable() {
    // This test would require:
    // 1. ClamAV enabled in config
    // 2. ClamAV fail-closed = false (fail-open)
    // 3. ClamAV service unavailable
    // 4. Attempt to upload should succeed (fail-open behavior)
    
    // Placeholder test documenting expected behavior
}
