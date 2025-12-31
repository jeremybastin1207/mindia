# Test Coverage

## Overview

This document describes the test coverage strategy, metrics, and targets for the Mindia project.

## Coverage Tools

### cargo-tarpaulin

We use [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) for code coverage analysis. It provides:

- Line coverage metrics
- Branch coverage (experimental)
- XML and HTML reports
- Integration with CI/CD

### Installation

```bash
cargo install cargo-tarpaulin --locked
```

## Coverage Targets

### Overall Target

**Target**: **70%+ line coverage** across all crates

### Per-Crate Targets

| Crate | Target | Priority |
|-------|--------|----------|
| `mindia-core` | 80%+ | High - Core domain logic |
| `mindia-db` | 75%+ | High - Data access layer |
| `mindia-api` | 70%+ | High - Main API and handlers |
| `mindia-worker` | 70%+ | High - Task queue infrastructure |
| `mindia-services` | 65%+ | Medium - External services |
| `mindia-infra` | 65%+ | Medium - Infrastructure code |
| `mindia-storage` | 70%+ | Medium - Storage abstraction |
| `mindia-processing` | 65%+ | Medium - Media processing |
| `mindia-plugins` | 60%+ | Low - Optional features |

## Running Coverage Locally

### Basic Coverage Report

```bash
# Run coverage for all crates
cargo tarpaulin --all-features

# Run coverage for specific crate
cargo tarpaulin --package mindia-api --all-features

# Generate HTML report
cargo tarpaulin --all-features --out Html --output-dir coverage
```

### Coverage with Tests

```bash
# Run tests with coverage
cargo tarpaulin --all-features --tests

# Include integration tests
cargo tarpaulin --all-features --tests --test integration
```

### View Coverage Report

After generating HTML report:

```bash
# Open in browser
open coverage/tarpaulin-report.html
# or
xdg-open coverage/tarpaulin-report.html
```

## CI/CD Integration

### GitLab CI/CD

Coverage is automatically calculated in the `test-coverage` job:

- Runs on merge requests, main, and develop branches
- Generates XML report (Cobertura format) for GitLab integration
- Coverage percentage displayed in merge request
- Coverage reports stored as artifacts (30 days retention)

### Coverage Badge

Coverage percentage is displayed in:
- GitLab merge request widgets
- Pipeline status
- Coverage reports artifact

## Coverage Metrics

### Current Coverage

Run coverage locally to see current metrics:

```bash
cargo tarpaulin --all-features --out Stdout
```

### Coverage Trends

Monitor coverage trends over time:
- Review coverage reports in CI/CD artifacts
- Track coverage percentage in merge requests
- Set up coverage trend tracking (if using external tools)

## Improving Coverage

### Priority Areas

1. **Critical Paths**: Upload handlers, authentication, database operations
2. **Error Handling**: All error paths should be tested
3. **Edge Cases**: Boundary conditions, invalid inputs
4. **Integration Tests**: End-to-end workflows

### Coverage Gaps

Common areas with low coverage:
- Error handling paths
- Optional feature code (feature-gated)
- Background job handlers
- Complex business logic

### Writing Tests for Coverage

1. **Test Happy Path**: Normal operation
2. **Test Error Cases**: All error conditions
3. **Test Edge Cases**: Boundary values, empty inputs
4. **Test Integration**: Multi-step workflows

## Coverage Exclusions

Some code may be excluded from coverage:

- Generated code (if any)
- Test utilities
- Main functions (entry points)
- Feature-gated code not enabled in test build

To exclude code from coverage:

```rust
#[cfg(not(tarpaulin_include))]
fn excluded_function() {
    // This won't be counted in coverage
}
```

## Best Practices

1. **Aim for 70%+ coverage**: Focus on critical paths first
2. **Don't chase 100%**: Some code (like main functions) doesn't need testing
3. **Test behavior, not implementation**: Focus on what code does, not how
4. **Coverage is a tool, not a goal**: High coverage doesn't guarantee quality
5. **Review coverage reports**: Identify untested critical paths

## Coverage Reports

### HTML Report

Generate detailed HTML report:

```bash
cargo tarpaulin --all-features --out Html --output-dir coverage
```

View `coverage/tarpaulin-report.html` in browser for:
- Line-by-line coverage
- File-level coverage percentages
- Uncovered lines highlighted

### XML Report (Cobertura)

Generate XML report for CI/CD integration:

```bash
cargo tarpaulin --all-features --out Xml --output-dir coverage
```

### JSON Report

Generate JSON report for programmatic analysis:

```bash
cargo tarpaulin --all-features --out Json --output-dir coverage
```

## Troubleshooting

### Coverage Not Accurate

- Ensure all features are enabled: `--all-features`
- Check that tests actually run: `cargo test` should pass
- Verify database is available for integration tests

### Coverage Job Fails

- Check that cargo-tarpaulin is installed
- Verify database service is running in CI
- Check for compilation errors

### Low Coverage

1. Identify uncovered files: Review HTML report
2. Prioritize critical paths: Focus on high-impact code
3. Add integration tests: Test complete workflows
4. Test error cases: Don't just test happy paths

## Future Improvements

1. **Branch Coverage**: Enable branch coverage when stable
2. **Coverage Trends**: Set up trend tracking over time
3. **Coverage Gates**: Fail CI if coverage drops below threshold
4. **Coverage Dashboard**: Visualize coverage across crates
5. **Automated Coverage Reports**: Generate reports on schedule

## Related Documentation

- [Testing Strategy](testing.md) - Overall testing approach
- [Development Setup](development-setup.md) - Local development environment
