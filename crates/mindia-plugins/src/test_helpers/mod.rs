//! Test helpers for plugin unit tests
//!
//! This module provides mock implementations of Storage and test fixtures
//! for isolated unit testing of plugins.

pub mod fixtures;
pub mod mock_repositories;
pub mod mock_storage;

pub use fixtures::*;
pub use mock_repositories::TestPluginContextBuilder;
pub use mock_storage::*;

use std::sync::Arc;
use uuid::Uuid;

/// Create a mock PluginContext for testing
///
/// This creates a context with mocked storage and repositories.
/// No database connection is needed, making it suitable for unit tests.
pub fn create_mock_plugin_context(
    tenant_id: Uuid,
    media_id: Uuid,
    config: serde_json::Value,
) -> crate::plugin::PluginContext {
    let storage = Arc::new(MockStorage::new()) as Arc<dyn mindia_storage::Storage>;
    let media_repo = Arc::new(mock_repositories::MockMediaRepository::new());
    let file_group_repo = Arc::new(mock_repositories::MockFileGroupRepository::new());

    crate::plugin::PluginContext {
        tenant_id,
        media_id,
        storage,
        media_repo,
        file_group_repo,
        get_public_file_url: None,
        config,
    }
}
