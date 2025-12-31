//! Plugin system – re-exports from mindia-plugins plus API-specific service.

#[cfg(feature = "plugin")]
pub use mindia_plugins::{
    PluginContext, PluginExecutionStatus, PluginInfo, PluginRegistry, PluginResult,
};

#[cfg(feature = "plugin")]
pub mod impls;
#[cfg(feature = "plugin")]
pub mod service;

#[cfg(feature = "plugin")]
pub use service::PluginService;
