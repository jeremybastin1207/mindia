//! Video processor - metadata extraction and validation

use crate::metadata::VideoMetadata;
use crate::traits::MediaProcessor;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Validate that a path doesn't contain shell metacharacters or dangerous sequences
fn validate_path(path: &str) -> Result<()> {
    let dangerous_chars = [';', '|', '&', '$', '`', '(', ')', '<', '>', '\n', '\r'];
    if path.chars().any(|c| dangerous_chars.contains(&c)) {
        return Err(anyhow!("Path contains dangerous characters: {}", path));
    }

    if path.contains("..") {
        return Err(anyhow!("Path contains directory traversal: {}", path));
    }

    Ok(())
}

/// Validate and canonicalize a file path to prevent directory traversal
fn validate_and_canonicalize_path(path: &Path) -> Result<PathBuf> {
    let path_str = path.to_string_lossy();
    validate_path(&path_str)?;

    if path.exists() {
        path.canonicalize()
            .map_err(|e| anyhow!("Failed to canonicalize path: {}", e))
    } else {
        if let Some(parent) = path.parent() {
            parent
                .canonicalize()
                .map_err(|e| anyhow!("Failed to canonicalize parent path: {}", e))?;
        }
        Ok(path.to_path_buf())
    }
}

pub struct VideoProcessor {
    ffprobe_path: String,
}

impl VideoProcessor {
    pub fn new(ffprobe_path: String) -> Result<Self> {
        validate_path(&ffprobe_path)
            .context("Invalid ffprobe_path: contains dangerous characters")?;

        if !ffprobe_path.chars().all(|c| {
            c.is_alphanumeric() || c == '/' || c == '-' || c == '_' || c == '.' || c == '\\'
        }) {
            return Err(anyhow!("Invalid ffprobe_path: contains unsafe characters"));
        }

        Ok(Self { ffprobe_path })
    }
}

#[async_trait]
impl MediaProcessor for VideoProcessor {
    type Metadata = VideoMetadata;

    async fn extract_metadata(&self, data: &[u8]) -> Result<Self::Metadata, anyhow::Error> {
        // Write data to temporary file for ffprobe
        let temp_file = tempfile::NamedTempFile::new()?;
        tokio::fs::write(temp_file.path(), data).await?;

        self.extract_metadata_from_path(temp_file.path()).await
    }

    fn validate(&self, data: &[u8]) -> Result<(), anyhow::Error> {
        // Basic validation - check for common video file signatures
        if data.len() < 12 {
            return Err(anyhow!("File too small to be a valid video"));
        }

        // Check for common video container signatures
        let header = &data[0..12];
        if header.starts_with(b"ftyp") || // MP4
            header[4..8] == *b"ftyp" ||
            header.starts_with(b"RIFF") || // AVI
            header[8..12] == *b"AVI " ||
            header.starts_with(b"\x1a\x45\xdf\xa3") || // Matroska/MKV
            header.starts_with(b"OggS")
        // OGG
        {
            Ok(())
        } else {
            // For now, accept any file and let ffprobe handle format detection
            Ok(())
        }
    }

    fn get_dimensions(&self, _data: &[u8]) -> Option<(u32, u32)> {
        // For dimensions, we'd need to extract metadata
        // This is a quick check that would require async, so return None
        // and let callers use extract_metadata if they need dimensions
        None
    }
}

impl VideoProcessor {
    /// Extract metadata from file path (for existing files)
    #[tracing::instrument(skip(self), fields(
        process.executable.name = "ffprobe",
        process.executable.path = %self.ffprobe_path,
        process.command = "ffprobe",
        ffmpeg.operation = "probe"
    ))]
    pub async fn extract_metadata_from_path(&self, video_path: &Path) -> Result<VideoMetadata> {
        let start = std::time::Instant::now();

        let validated_path =
            validate_and_canonicalize_path(video_path).context("Invalid video path")?;

        let output = Command::new("ffprobe")
            .args([
                "-v",
                "quiet",
                "-print_format",
                "json",
                "-show_format",
                "-show_streams",
                "-select_streams",
                "v:0",
            ])
            .arg(&validated_path)
            .output()
            .await
            .context("Failed to execute ffprobe")?;

        if !output.status.success() {
            return Err(anyhow!(
                "ffprobe failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let probe_data: serde_json::Value =
            serde_json::from_slice(&output.stdout).context("Failed to parse ffprobe output")?;

        let stream = probe_data["streams"]
            .get(0)
            .ok_or_else(|| anyhow!("No video stream found"))?;

        let format = &probe_data["format"];

        let duration = format["duration"]
            .as_str()
            .and_then(|d| d.parse::<f64>().ok())
            .ok_or_else(|| anyhow!("Could not parse duration"))?;

        let width = stream["width"]
            .as_u64()
            .ok_or_else(|| anyhow!("Could not parse width"))? as u32;

        let height = stream["height"]
            .as_u64()
            .ok_or_else(|| anyhow!("Could not parse height"))? as u32;

        let codec = stream["codec_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let bitrate = format["bit_rate"]
            .as_str()
            .and_then(|b| b.parse::<u64>().ok());

        let framerate = stream["r_frame_rate"].as_str().and_then(|r| {
            let parts: Vec<&str> = r.split('/').collect();
            if parts.len() == 2 {
                let num: f32 = parts[0].parse().ok()?;
                let den: f32 = parts[1].parse().ok()?;
                if den != 0.0 {
                    Some(num / den)
                } else {
                    None
                }
            } else {
                None
            }
        });

        let elapsed = start.elapsed();
        tracing::info!(
            duration_ms = elapsed.as_millis(),
            video_duration = duration,
            width = width,
            height = height,
            codec = %codec,
            "Video probe completed"
        );

        Ok(VideoMetadata {
            duration,
            width,
            height,
            codec,
            bitrate,
            framerate,
        })
    }
}
