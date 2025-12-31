//! Mindia Plugins
//!
//! This crate provides the plugin system infrastructure and implementations.
//! Plugins allow extending Mindia's functionality with third-party service integrations.

#[cfg(feature = "plugin")]
pub mod plugin;
#[cfg(feature = "plugin")]
pub mod registry;
#[cfg(feature = "plugin")]
pub mod validation;

#[cfg(feature = "plugin-assembly-ai")]
pub mod assembly_ai;
#[cfg(all(feature = "plugin", feature = "plugin-aws-rekognition"))]
pub mod aws_rekognition;
#[cfg(all(feature = "plugin", feature = "plugin-aws-rekognition-moderation"))]
pub mod aws_rekognition_moderation;
#[cfg(feature = "plugin-aws-transcribe")]
pub mod aws_transcribe;
#[cfg(feature = "plugin-claude-vision")]
pub mod claude_vision;
#[cfg(feature = "plugin-google-vision")]
pub mod google_vision;
#[cfg(feature = "plugin-replicate-deoldify")]
pub mod replicate_deoldify;

// Re-export commonly used types
#[cfg(feature = "plugin")]
pub use plugin::{
    Plugin, PluginContext, PluginExecutionStatus, PluginInfo, PluginResult, PluginUsage,
};
#[cfg(feature = "plugin")]
pub use registry::PluginRegistry;
#[cfg(feature = "plugin")]
pub use validation::{
    validate_audio_size, validate_image_size, validate_size, validate_video_size,
};

// Alias for plugins (plural) to match existing imports
#[cfg(feature = "plugin")]
pub mod plugins {
    pub use super::plugin::*;
}

// Re-export plugin implementations
#[cfg(feature = "plugin-assembly-ai")]
pub use assembly_ai::AssemblyAiPlugin;
#[cfg(all(feature = "plugin", feature = "plugin-aws-rekognition"))]
pub use aws_rekognition::AwsRekognitionPlugin;
#[cfg(all(feature = "plugin", feature = "plugin-aws-rekognition-moderation"))]
pub use aws_rekognition_moderation::AwsRekognitionModerationPlugin;
#[cfg(feature = "plugin-aws-transcribe")]
pub use aws_transcribe::AwsTranscribePlugin;
#[cfg(feature = "plugin-claude-vision")]
pub use claude_vision::ClaudeVisionPlugin;
#[cfg(feature = "plugin-google-vision")]
pub use google_vision::GoogleVisionPlugin;
#[cfg(feature = "plugin-replicate-deoldify")]
pub use replicate_deoldify::ReplicateDeoldifyPlugin;

// Test helpers (only available in test mode)
#[cfg(test)]
pub mod test_helpers;
