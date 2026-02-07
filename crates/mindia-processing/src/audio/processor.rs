//! Audio processor - metadata extraction and validation

use crate::metadata::AudioMetadata as UnifiedAudioMetadata;
use crate::traits::MediaProcessor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Command;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
struct FFprobeOutput {
    format: Option<FFprobeFormat>,
    streams: Option<Vec<FFprobeStream>>,
}

#[derive(Debug, Deserialize)]
struct FFprobeFormat {
    duration: Option<String>,
    bit_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FFprobeStream {
    codec_type: Option<String>,
    sample_rate: Option<String>,
    channels: Option<i32>,
    codec_name: Option<String>,
}

pub struct AudioProcessor {
    ffprobe_path: String,
}

impl AudioProcessor {
    pub fn new(ffprobe_path: String) -> Self {
        Self { ffprobe_path }
    }

    /// Extract audio metadata from a file using ffprobe (path-based)
    #[tracing::instrument(skip(self, file_path), fields(service = "audio"))]
    pub async fn extract_audio_metadata_from_path(
        &self,
        file_path: &str,
    ) -> Result<UnifiedAudioMetadata, anyhow::Error> {
        info!("Extracting audio metadata from: {}", file_path);

        let output = Command::new(&self.ffprobe_path)
            .args([
                "-v",
                "error",
                "-show_format",
                "-show_streams",
                "-of",
                "json",
                file_path,
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run ffprobe: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("ffprobe failed: {}", stderr);
            return Err(anyhow::anyhow!("ffprobe failed: {}", stderr));
        }

        let json_output: FFprobeOutput = serde_json::from_slice(&output.stdout)
            .map_err(|e| anyhow::anyhow!("Failed to parse ffprobe output: {}", e))?;

        let duration = json_output
            .format
            .as_ref()
            .and_then(|f| f.duration.as_ref())
            .and_then(|d| d.parse::<f64>().ok());

        let bitrate = json_output
            .format
            .as_ref()
            .and_then(|f| f.bit_rate.as_ref())
            .and_then(|b| b.parse::<i32>().ok());

        let audio_stream = json_output.streams.and_then(|streams| {
            streams
                .into_iter()
                .find(|s| s.codec_type.as_deref() == Some("audio"))
        });

        let sample_rate = audio_stream
            .as_ref()
            .and_then(|s| s.sample_rate.as_ref())
            .and_then(|sr| sr.parse::<i32>().ok());

        let channels = audio_stream.as_ref().and_then(|s| s.channels);

        let codec = audio_stream.as_ref().and_then(|s| s.codec_name.clone());

        Ok(UnifiedAudioMetadata {
            duration,
            bitrate,
            sample_rate,
            channels,
            codec,
        })
    }
}

#[async_trait]
impl MediaProcessor for AudioProcessor {
    type Metadata = UnifiedAudioMetadata;

    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, anyhow::Error> {
        // Write data to temporary file for ffprobe
        let temp_file = tempfile::NamedTempFile::new()?;
        tokio::fs::write(temp_file.path(), data).await?;

        self.extract_audio_metadata_from_path(
            temp_file
                .path()
                .to_str()
                .ok_or_else(|| anyhow!("Invalid temp file path"))?,
        )
        .await
    }

    fn validate(&self, data: &[u8]) -> Result<(), anyhow::Error> {
        // Basic validation - check for common audio file signatures
        if data.len() < 4 {
            return Err(anyhow!("File too small to be a valid audio file"));
        }

        // Check for common audio file signatures
        let header = &data[0..4.min(data.len())];
        if header.starts_with(b"RIFF") || // WAV
            header.starts_with(b"OggS") || // OGG
            header.starts_with(b"fLaC") || // FLAC
            (data.len() >= 3 && &data[0..3] == b"ID3") || // MP3
            (data.len() >= 11 && &data[4..11] == b"ftypM4A")
        // M4A
        {
            Ok(())
        } else {
            // For now, accept any file and let ffprobe handle format detection
            Ok(())
        }
    }

    fn get_dimensions(&self, _data: &[u8]) -> Option<(u32, u32)> {
        // Audio files don't have dimensions
        None
    }
}

pub type AudioService = AudioProcessor;

pub use crate::metadata::AudioMetadata;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_processor() -> AudioProcessor {
        AudioProcessor::new("ffprobe".to_string())
    }

    #[test]
    fn test_validate_wav() {
        let processor = create_test_processor();
        // WAV file signature: "RIFF"
        let wav_data = b"RIFF\x00\x00\x00\x00WAVE";
        assert!(processor.validate(wav_data).is_ok());
    }

    #[test]
    fn test_validate_ogg() {
        let processor = create_test_processor();
        // OGG file signature: "OggS"
        let ogg_data = b"OggS\x00\x00\x00\x00";
        assert!(processor.validate(ogg_data).is_ok());
    }

    #[test]
    fn test_validate_flac() {
        let processor = create_test_processor();
        // FLAC file signature: "fLaC"
        let flac_data = b"fLaC\x00\x00\x00\x00";
        assert!(processor.validate(flac_data).is_ok());
    }

    #[test]
    fn test_validate_mp3() {
        let processor = create_test_processor();
        // MP3 file signature: "ID3"
        let mp3_data = b"ID3\x03\x00\x00\x00";
        assert!(processor.validate(mp3_data).is_ok());
    }

    #[test]
    fn test_validate_m4a() {
        let processor = create_test_processor();
        // M4A file signature: "ftypM4A" at offset 4
        let mut m4a_data = vec![0u8; 20];
        m4a_data[4..11].copy_from_slice(b"ftypM4A");
        assert!(processor.validate(&m4a_data).is_ok());
    }

    #[test]
    fn test_validate_too_small() {
        let processor = create_test_processor();
        let small_data = b"ABC";
        assert!(processor.validate(small_data).is_err());
    }

    #[test]
    fn test_validate_empty() {
        let processor = create_test_processor();
        let empty_data = b"";
        assert!(processor.validate(empty_data).is_err());
    }

    #[test]
    fn test_validate_unknown_format() {
        let processor = create_test_processor();
        // Unknown format - should still pass (ffprobe will handle it)
        let unknown_data = b"UNKNOWN_FORMAT_DATA_HERE";
        // Currently accepts any file and lets ffprobe handle it
        assert!(processor.validate(unknown_data).is_ok());
    }

    #[test]
    fn test_get_dimensions() {
        let processor = create_test_processor();
        let audio_data = b"RIFF\x00\x00\x00\x00WAVE";
        // Audio files don't have dimensions
        assert_eq!(processor.get_dimensions(audio_data), None);
    }

    #[test]
    fn test_processor_new() {
        let processor = AudioProcessor::new("custom_ffprobe_path".to_string());
        // Just verify it creates without panicking
        assert_eq!(processor.ffprobe_path, "custom_ffprobe_path");
    }
}
