# Testing Strategy

This document outlines the testing strategy for the Mindia project, including integration tests, load tests, and performance benchmarks.

## Overview

The test suite includes:

1. **Unit Tests** - Test individual functions and components in isolation
2. **Integration Tests** - Test complete workflows and system behavior
3. **Benchmarks** - Performance benchmarks for critical operations (optional)

## Test Structure

```
tests/
├── helpers/           # Test utilities and helpers
│   ├── auth.rs       # Authentication helpers
│   ├── storage.rs    # Storage helpers
│   ├── workflows.rs  # Workflow helpers
│   └── fixtures.rs   # Test data generation
├── integration/       # Integration tests
│   ├── workflow_test.rs
│   ├── rate_limit_test.rs
│   ├── concurrent_test.rs
│   └── error_scenarios_test.rs
└── fixtures/         # Test fixture files
```

## Running Tests

### Unit Tests

```bash
cargo test --lib
```

Run specific test:

```bash
cargo test test_name
```

### Integration Tests

```bash
cargo test --test integration
```

Run specific integration test file:

```bash
cargo test --test workflow_test
```

### All Tests

```bash
cargo test
```

## Test Helpers

### Workflow Helpers

Located in `tests/helpers/workflows.rs`, these helpers provide reusable functions for common workflows:

- `upload_and_verify_image_workflow()` - Complete image workflow (Upload → Get → Transform → Delete)
- `video_upload_and_stream_workflow()` - Video processing workflow
- `multi_user_workflow()` - Multiple users uploading simultaneously
- `rate_limit_verification()` - Rate limit enforcement testing
- `webhook_delivery_workflow()` - Webhook creation and delivery
- `search_workflow()` - Search functionality testing
- `batch_upload_workflow()` - Batch operations

### Fixture Helpers

Located in `tests/helpers/fixtures.rs`, these helpers generate test data:

- `create_test_png()` - Generate test PNG images
- `create_test_pdf()` - Generate test PDF documents
- `create_test_video()` - Generate test video files
- `create_file_of_size()` - Create files of specific sizes for load testing

## Integration Test Scenarios

### Complete Workflows

1. **Image Workflow**: Register → Upload → Get → Transform → Download → Delete → Verify cleanup
2. **Video Processing**: Upload → Poll status → Wait for completion → Stream HLS → Verify variants
3. **Webhook Delivery**: Create webhook → Upload file → Verify event delivery
4. **Search Workflow**: Upload → Index → Search → Verify results

### Multi-Tenant Isolation

- Create two tenants
- Upload files to each
- Verify data isolation (each tenant only sees their own files)

### Rate Limiting

- Make requests up to limit
- Verify 429 responses with proper headers
- Wait for reset
- Verify recovery

### Error Handling

- Invalid authentication
- Invalid file types
- Missing files
- Network errors
- Cross-tenant access attempts

### Concurrent Operations

- Multiple users uploading simultaneously
- Concurrent file access
- Concurrent deletions
- Mixed operations under load

## CI/CD Integration

Tests run automatically in CI/CD:

- **On every push/PR**: Unit tests and integration tests
- **On demand**: Can be triggered manually via workflow_dispatch

## Best Practices

### Writing Tests

1. **Use Test Helpers**: Leverage workflow and fixture helpers for consistency
2. **Isolate Tests**: Each test should be independent and not rely on others
3. **Clean Up**: Delete test data after tests complete
4. **Use Descriptive Names**: Test names should clearly describe what they test
5. **Test Error Cases**: Include tests for error scenarios and edge cases

### Debugging Tests

1. **Use `--nocapture`**: See println! output in tests
2. **Check Logs**: Enable RUST_LOG=debug for detailed logging
3. **Use `--test-threads=1`**: Run tests sequentially for debugging
4. **Isolate Failing Test**: Run only the failing test to debug
5. **Check Test State**: Verify test database and cleanup state

## Troubleshooting

### Common Issues

**Tests failing with connection errors**

- Check if testcontainers is running properly
- Verify Docker is accessible
- Check database connection string

**Rate limit tests not working**

- Verify rate limiting middleware is enabled in test environment
- Check rate limit configuration matches test expectations
- Ensure rate limit headers are being set

### Getting Help

- Check test logs for detailed error messages
- Review CI/CD logs for integration test failures
- Review test helper documentation for usage examples

## Adding New Tests

### Adding Integration Tests

1. Create test file in `tests/integration/`
2. Add module to `tests/integration/mod.rs`
3. Use test helpers from `tests/helpers/`
4. Follow existing test patterns

Example:

```rust
mod helpers;

use helpers::auth::register_test_user;
use helpers::setup_test_app;

#[tokio::test]
async fn test_my_feature() {
    let app = setup_test_app().await;
    let client = app.client();
    let user = register_test_user(client, None, None, None).await;
    
    // Your test code here
}
```

## Success Criteria

All tests should meet these criteria:

- ✅ All unit tests pass consistently
- ✅ All integration tests pass consistently
- ✅ Rate limits enforced correctly
- ✅ Tests run in CI/CD pipeline
- ✅ Documentation is complete and up-to-date
