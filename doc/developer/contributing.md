# Contributing

Guidelines for contributing to Mindia.

## Getting Started

1. Fork the repository
2. Clone your fork
3. Create a feature branch
4. Make your changes
5. Open a Pull Request

## Development Setup

See [Development Setup](development-setup.md) for detailed instructions.

```bash
# Quick start
git clone <your-fork>
cd mindia
cp .env.example .env
cargo run -p mindia-api
```

## Code Style

### Formatting

```bash
# Format all code before committing
cargo fmt

# Check formatting
cargo fmt --check
```

### Linting

```bash
# Run clippy
cargo clippy -- -D warnings

# Fix automatic issues
cargo clippy --fix
```

### Code Quality

- Write idiomatic Rust
- Use meaningful variable names
- Add comments for complex logic
- Keep functions small and focused
- Avoid unwrap() in production code

## Testing

### Run Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture
```

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        let input = create_test_input();

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected_value);
    }

    #[tokio::test]
    async fn test_async_function() {
        let result = async_function().await.unwrap();
        assert!(result.is_ok());
    }
}
```

## Commit & Release Guidelines

### Commit Messages

Follow conventional commits:

```
type(scope): brief description

Longer explanation if needed.

Fixes #123
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

**Examples**:
```
feat(images): add WebP support
fix(auth): resolve token expiry bug
docs(api): update authentication docs
refactor(db): improve query performance
```

### Release-related Commits

- Use `chore(release): bump version to X.Y.Z` for version bump commits created by `scripts/bump-version.sh`.
- Document user-visible changes in `CHANGELOG.md` as part of your PR when appropriate.

For the full release process (including versioning, tagging, and GitHub Releases), see:

- [`Releasing Mindia`](releasing.md)

### Branch Names

```
feature/add-webp-support
fix/token-expiry-bug
docs/update-api-docs
refactor/optimize-queries
```

## Pull Request Process

1. **Update tests**: Add/update tests for your changes
2. **Update docs**: Document new features/changes
3. **Run checks**: Ensure `cargo test`, `cargo fmt`, `cargo clippy` pass
4. **Describe changes**: Write a clear PR description
5. **Link issues**: Reference related issues

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
How was this tested?

## Checklist
- [ ] Tests pass
- [ ] Code formatted (cargo fmt)
- [ ] Linted (cargo clippy)
- [ ] Documentation updated
```

## Code Review

### For Contributors

- Be open to feedback
- Respond to comments
- Update based on reviews
- Keep PRs focused and small

### For Reviewers

- Be constructive and kind
- Explain reasoning
- Approve when ready
- Suggest improvements

## Areas for Contribution

### Good First Issues

- Documentation improvements
- Test coverage
- Error messages
- Code comments

### Feature Ideas

- New image formats
- Additional transformations
- Storage backends
- Authentication providers
- API endpoints

### Bug Fixes

Check GitHub issues for bugs to fix.

## Documentation

### Code Comments

```rust
/// Resizes an image to the specified dimensions.
///
/// # Arguments
/// * `image` - The source image
/// * `width` - Target width in pixels
/// * `height` - Target height in pixels
///
/// # Returns
/// Resized image buffer
///
/// # Errors
/// Returns error if dimensions are invalid
pub fn resize_image(
    image: &DynamicImage,
    width: u32,
    height: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>> {
    // Implementation
}
```

### Documentation Updates

When adding features:
- Update relevant docs in `doc/`
- Add examples
- Update API reference
- Add changelog entry

## Questions?

- Open a [GitHub issue](https://github.com/jeremybastin1207/mindia/issues)
- Email maintainers

## License

By contributing, you agree your contributions will be licensed under the same license as the project.

## Next Steps

- [Development Setup](development-setup.md) - Set up environment
- [Code Structure](code-structure.md) - Understand codebase
- [Architecture](architecture.md) - System design

