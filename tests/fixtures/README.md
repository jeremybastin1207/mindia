# Test Fixtures

This directory contains test fixture files for integration and load testing.

## Structure

- `sample_images/` - Test image files (PNG, JPEG, etc.)
- `sample_videos/` - Test video files (MP4, MOV, etc.)
- `sample_documents/` - Test document files (PDF, etc.)

## Usage

Most test fixtures are generated programmatically in `tests/helpers/fixtures.rs`. However, this directory can be used to store:

1. **Large test files** that are impractical to generate on-the-fly
2. **Real-world test data** that should be committed to the repository
3. **Reference files** for comparison testing

## Adding Fixtures

When adding new fixtures:

1. Keep files small (< 1MB) to avoid bloating the repository
2. Use `.gitignore` for large files or generated data
3. Document the purpose and format of each fixture
4. Use descriptive names (e.g., `test-image-100x100.png`)

## Note

For most tests, fixtures are generated programmatically using `helpers::fixtures` functions. This ensures tests are fast and don't depend on external files.
