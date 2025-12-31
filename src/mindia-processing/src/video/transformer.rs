//! Video transformer - video transformations

use crate::traits::{MediaTransformer, TransformType};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTransformOptions {
    pub transform_type: VideoTransformType,
    pub params: VideoTransformParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoTransformType {
    Transcode {
        output_format: String, // mp4, webm, etc.
    },
    Scale {
        width: Option<u32>,
        height: Option<u32>,
    },
    Trim {
        start_seconds: f64,
        end_seconds: Option<f64>,
    },
    ExtractThumbnail {
        timestamp_seconds: f64,
    },
    ExtractAudio {
        output_format: String, // mp3, aac, wav
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoTransformParams {
    pub bitrate_kbps: Option<u32>,
    pub quality: Option<String>, // "low", "medium", "high"
}

pub struct VideoTransformer {
    ffmpeg_path: String,
}

impl VideoTransformer {
    pub fn new(ffmpeg_path: String) -> Result<Self> {
        // Validate ffmpeg_path
        let dangerous_chars = [';', '|', '&', '$', '`', '(', ')', '<', '>', '\n', '\r'];
        if ffmpeg_path.chars().any(|c| dangerous_chars.contains(&c)) {
            return Err(anyhow!(
                "Invalid ffmpeg_path: contains dangerous characters"
            ));
        }

        Ok(Self { ffmpeg_path })
    }
}

#[async_trait]
impl MediaTransformer for VideoTransformer {
    type Options = VideoTransformOptions;

    async fn transform(&self, data: &[u8], options: Self::Options) -> Result<Bytes, anyhow::Error> {
        // Write input to temp file
        let input_temp = tempfile::NamedTempFile::new()?;
        tokio::fs::write(input_temp.path(), data).await?;

        // Create output temp file
        let output_temp = tempfile::NamedTempFile::new()?;
        let output_path = output_temp.path();

        match options.transform_type {
            VideoTransformType::Transcode { output_format } => {
                self.transcode(
                    input_temp.path(),
                    output_path,
                    &output_format,
                    options.params,
                )
                .await?;
            }
            VideoTransformType::Scale { width, height } => {
                self.scale(
                    input_temp.path(),
                    output_path,
                    width,
                    height,
                    options.params,
                )
                .await?;
            }
            VideoTransformType::Trim {
                start_seconds,
                end_seconds,
            } => {
                self.trim(
                    input_temp.path(),
                    output_path,
                    start_seconds,
                    end_seconds,
                    options.params,
                )
                .await?;
            }
            VideoTransformType::ExtractThumbnail { timestamp_seconds } => {
                self.extract_thumbnail(input_temp.path(), output_path, timestamp_seconds)
                    .await?;
            }
            VideoTransformType::ExtractAudio { output_format } => {
                self.extract_audio(input_temp.path(), output_path, &output_format)
                    .await?;
            }
        }

        // Read output file
        let output_data = tokio::fs::read(output_path).await?;
        Ok(Bytes::from(output_data))
    }

    fn supported_transforms(&self) -> Vec<TransformType> {
        vec![
            TransformType::VideoTranscode,
            TransformType::VideoScale,
            TransformType::VideoTrim,
            TransformType::VideoThumbnail,
            TransformType::VideoExtractAudio,
        ]
    }
}

impl VideoTransformer {
    async fn transcode(
        &self,
        input_path: &Path,
        output_path: &Path,
        format: &str,
        params: VideoTransformParams,
    ) -> Result<()> {
        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

        // Set output format
        args.extend_from_slice(&["-f".to_string(), format.to_string()]);

        // Set video codec
        args.extend_from_slice(&["-c:v".to_string(), "libx264".to_string()]);

        // Set bitrate if specified
        if let Some(bitrate) = params.bitrate_kbps {
            args.extend_from_slice(&["-b:v".to_string(), format!("{}k", bitrate)]);
        }

        // Set quality preset
        if let Some(quality) = params.quality {
            args.extend_from_slice(&["-preset".to_string(), quality]);
        }

        args.push(output_path.to_string_lossy().to_string());

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg transcode failed: {}", stderr));
        }

        Ok(())
    }

    async fn scale(
        &self,
        input_path: &Path,
        output_path: &Path,
        width: Option<u32>,
        height: Option<u32>,
        params: VideoTransformParams,
    ) -> Result<()> {
        let scale_filter = match (width, height) {
            (Some(w), Some(h)) => format!("scale={}:{}", w, h),
            (Some(w), None) => format!("scale={}:-1", w),
            (None, Some(h)) => format!("scale=-1:{}", h),
            (None, None) => return Err(anyhow!("Width or height must be specified")),
        };

        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];
        args.extend_from_slice(&["-vf".to_string(), scale_filter]);
        args.extend_from_slice(&["-c:v".to_string(), "libx264".to_string()]);

        if let Some(bitrate) = params.bitrate_kbps {
            args.extend_from_slice(&["-b:v".to_string(), format!("{}k", bitrate)]);
        }

        args.push(output_path.to_string_lossy().to_string());

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg scale failed: {}", stderr));
        }

        Ok(())
    }

    async fn trim(
        &self,
        input_path: &Path,
        output_path: &Path,
        start: f64,
        end: Option<f64>,
        _params: VideoTransformParams,
    ) -> Result<()> {
        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

        // Seek to start
        args.extend_from_slice(&["-ss".to_string(), start.to_string()]);

        // Set duration if end is specified
        if let Some(end_time) = end {
            let duration = end_time - start;
            args.extend_from_slice(&["-t".to_string(), duration.to_string()]);
        }

        args.extend_from_slice(&["-c".to_string(), "copy".to_string()]);
        args.push(output_path.to_string_lossy().to_string());

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg trim failed: {}", stderr));
        }

        Ok(())
    }

    async fn extract_thumbnail(
        &self,
        input_path: &Path,
        output_path: &Path,
        timestamp: f64,
    ) -> Result<()> {
        let args = vec![
            "-ss".to_string(),
            timestamp.to_string(),
            "-i".to_string(),
            input_path.to_string_lossy().to_string(),
            "-vframes".to_string(),
            "1".to_string(),
            "-q:v".to_string(),
            "2".to_string(),
            "-y".to_string(),
            output_path.to_string_lossy().to_string(),
        ];

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg thumbnail extraction failed: {}", stderr));
        }

        Ok(())
    }

    async fn extract_audio(
        &self,
        input_path: &Path,
        output_path: &Path,
        format: &str,
    ) -> Result<()> {
        let args = vec![
            "-i".to_string(),
            input_path.to_string_lossy().to_string(),
            "-vn".to_string(), // No video
            "-acodec".to_string(),
            match format {
                "mp3" => "libmp3lame",
                "aac" => "aac",
                "wav" => "pcm_s16le",
                _ => "libmp3lame",
            }
            .to_string(),
            "-f".to_string(),
            format.to_string(),
            "-y".to_string(),
            output_path.to_string_lossy().to_string(),
        ];

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg audio extraction failed: {}", stderr));
        }

        Ok(())
    }
}
