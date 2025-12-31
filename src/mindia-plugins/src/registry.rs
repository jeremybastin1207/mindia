//! Plugin registry for managing available plugins

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::plugin::{Plugin, PluginInfo};

/// Registry for managing and retrieving plugins.
///
/// Thread-safe and async-compatible using tokio's RwLock.
/// Multiple async tasks can read plugins simultaneously without blocking,
/// while write operations (registration) are serialized.
#[derive(Clone)]
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Arc<dyn Plugin>>>>,
    plugin_info: Arc<RwLock<HashMap<String, PluginInfo>>>,
}

impl PluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            plugin_info: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a plugin with the registry
    ///
    /// Thread-safe: Uses async RwLock for concurrent access.
    /// Multiple tasks can register plugins safely (though registration typically happens at startup).
    pub async fn register(&self, plugin: Arc<dyn Plugin>, info: PluginInfo) -> Result<()> {
        let name = plugin.name().to_string();

        // Acquire write locks for both maps (async, non-blocking)
        let mut plugins = self.plugins.write().await;
        let mut plugin_info = self.plugin_info.write().await;

        plugins.insert(name.clone(), plugin);
        plugin_info.insert(name, info);

        Ok(())
    }

    /// Get a plugin by name
    ///
    /// Thread-safe and async-compatible: Uses async RwLock for concurrent reads.
    /// Multiple async tasks can read plugins simultaneously without blocking.
    pub async fn get(&self, name: &str) -> Result<Arc<dyn Plugin>> {
        let plugins = self.plugins.read().await;

        plugins
            .get(name)
            .cloned()
            .with_context(|| format!("Plugin '{}' not found", name))
    }

    /// List all registered plugins
    ///
    /// Thread-safe and async-compatible: Uses async RwLock for concurrent reads.
    pub async fn list(&self) -> Result<Vec<PluginInfo>> {
        let plugin_info = self.plugin_info.read().await;

        Ok(plugin_info.values().cloned().collect())
    }

    /// Check if a plugin is registered
    ///
    /// Thread-safe and async-compatible: Uses async RwLock for concurrent reads.
    pub async fn contains(&self, name: &str) -> Result<bool> {
        let plugins = self.plugins.read().await;

        Ok(plugins.contains_key(name))
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{Plugin, PluginContext, PluginExecutionStatus, PluginResult};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;

    // Mock plugin for testing
    #[derive(Debug)]
    struct MockPlugin {
        name: String,
    }

    impl MockPlugin {
        fn new(name: impl Into<String>) -> Self {
            Self { name: name.into() }
        }
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn name(&self) -> &str {
            &self.name
        }

        async fn execute(&self, _context: PluginContext) -> Result<PluginResult> {
            Ok(PluginResult {
                status: PluginExecutionStatus::Success,
                data: json!({}),
                error: None,
                metadata: None,
                usage: None,
            })
        }

        fn validate_config(&self, _config: &serde_json::Value) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_new_registry_is_empty() {
        let registry = PluginRegistry::new();
        assert!(registry.list().await.unwrap().is_empty());
        assert!(!registry.contains("test_plugin").await.unwrap());
    }

    #[tokio::test]
    async fn test_default_registry_is_empty() {
        let registry = PluginRegistry::default();
        assert!(registry.list().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_register_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test_plugin"));
        let info = PluginInfo {
            name: "test_plugin".to_string(),
            description: "Test plugin".to_string(),
            supported_media_types: vec!["audio".to_string()],
        };

        registry.register(plugin, info.clone()).await.unwrap();

        assert!(registry.contains("test_plugin").await.unwrap());
        assert_eq!(registry.list().await.unwrap().len(), 1);
        assert_eq!(registry.list().await.unwrap()[0].name, "test_plugin");
    }

    #[tokio::test]
    async fn test_get_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test_plugin"));
        let info = PluginInfo {
            name: "test_plugin".to_string(),
            description: "Test plugin".to_string(),
            supported_media_types: vec!["audio".to_string()],
        };

        registry.register(plugin.clone(), info).await.unwrap();

        let retrieved = registry.get("test_plugin").await.unwrap();
        assert_eq!(retrieved.name(), "test_plugin");
    }

    #[tokio::test]
    async fn test_get_nonexistent_plugin() {
        let registry = PluginRegistry::new();
        let result = registry.get("nonexistent").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Plugin 'nonexistent' not found"));
    }

    #[tokio::test]
    async fn test_list_plugins() {
        let registry = PluginRegistry::new();

        // Register first plugin
        let plugin1 = Arc::new(MockPlugin::new("plugin1"));
        let info1 = PluginInfo {
            name: "plugin1".to_string(),
            description: "First plugin".to_string(),
            supported_media_types: vec!["audio".to_string()],
        };
        registry.register(plugin1, info1).await.unwrap();

        // Register second plugin
        let plugin2 = Arc::new(MockPlugin::new("plugin2"));
        let info2 = PluginInfo {
            name: "plugin2".to_string(),
            description: "Second plugin".to_string(),
            supported_media_types: vec!["video".to_string()],
        };
        registry.register(plugin2, info2).await.unwrap();

        let plugins = registry.list().await.unwrap();
        assert_eq!(plugins.len(), 2);

        let names: Vec<&String> = plugins.iter().map(|p| &p.name).collect();
        assert!(names.contains(&&"plugin1".to_string()));
        assert!(names.contains(&&"plugin2".to_string()));
    }

    #[tokio::test]
    async fn test_contains_plugin() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test_plugin"));
        let info = PluginInfo {
            name: "test_plugin".to_string(),
            description: "Test plugin".to_string(),
            supported_media_types: vec!["audio".to_string()],
        };

        assert!(!registry.contains("test_plugin").await.unwrap());
        registry.register(plugin, info).await.unwrap();
        assert!(registry.contains("test_plugin").await.unwrap());
        assert!(!registry.contains("other_plugin").await.unwrap());
    }

    #[tokio::test]
    async fn test_register_multiple_plugins() {
        let registry = PluginRegistry::new();

        for i in 0..5 {
            let plugin = Arc::new(MockPlugin::new(format!("plugin_{}", i)));
            let info = PluginInfo {
                name: format!("plugin_{}", i),
                description: format!("Plugin {}", i),
                supported_media_types: vec!["audio".to_string()],
            };
            registry.register(plugin, info).await.unwrap();
        }

        assert_eq!(registry.list().await.unwrap().len(), 5);
        for i in 0..5 {
            assert!(registry.contains(&format!("plugin_{}", i)).await.unwrap());
        }
    }

    #[tokio::test]
    async fn test_clone_registry() {
        let registry = PluginRegistry::new();
        let plugin = Arc::new(MockPlugin::new("test_plugin"));
        let info = PluginInfo {
            name: "test_plugin".to_string(),
            description: "Test plugin".to_string(),
            supported_media_types: vec!["audio".to_string()],
        };
        registry.register(plugin, info).await.unwrap();

        let cloned = registry.clone();
        assert!(cloned.contains("test_plugin").await.unwrap());
        assert_eq!(cloned.list().await.unwrap().len(), 1);
    }
}
