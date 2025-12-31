//! Plugin implementations

mod assembly_ai;
#[cfg(all(feature = "plugin", feature = "content-moderation"))]
mod aws_rekognition;
#[cfg(all(feature = "plugin", feature = "plugin-aws-transcribe"))]
mod aws_transcribe;
#[cfg(feature = "plugin")]
mod google_vision;

pub use assembly_ai::AssemblyAiPlugin;
#[cfg(all(feature = "plugin", feature = "content-moderation"))]
pub use aws_rekognition::AwsRekognitionPlugin;
#[cfg(all(feature = "plugin", feature = "plugin-aws-transcribe"))]
pub use aws_transcribe::AwsTranscribePlugin;
#[cfg(feature = "plugin")]
pub use google_vision::GoogleVisionPlugin;

