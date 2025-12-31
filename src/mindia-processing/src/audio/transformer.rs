//! Audio transformer - audio transformations

use crate::traits::{MediaTransformer, TransformType};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTransformOptions {
    pub transform_type: AudioTransformType,
    pub params: AudioTransformParams,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioTransformType {
    Transcode {
        output_format: String, // mp3, aac, wav, ogg
    },
    Trim {
        start_seconds: f64,
        end_seconds: Option<f64>,
    },
    Normalize,
    ChangeBitrate {
        bitrate_kbps: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTransformParams {
    pub quality: Option<String>, // "low", "medium", "high"
}

pub struct AudioTransformer {
    ffmpeg_path: String,
}

impl AudioTransformer {
    pub fn new(ffmpeg_path: String) -> Result<Self> {
        Ok(Self { ffmpeg_path })
    }
}

#[async_trait]
impl MediaTransformer for AudioTransformer {
    type Options = AudioTransformOptions;

    async fn transform(&self, data: &[u8], options: Self::Options) -> Result<Bytes, anyhow::Error> {
        // Write input to temp file
        let input_temp = tempfile::NamedTempFile::new()?;
        tokio::fs::write(input_temp.path(), data).await?;

        // Create output temp file
        let output_temp = tempfile::NamedTempFile::new()?;
        let output_path = output_temp.path();

        match options.transform_type {
            AudioTransformType::Transcode { output_format } => {
                self.transcode(
                    input_temp.path(),
                    output_path,
                    &output_format,
                    options.params,
                )
                .await?;
            }
            AudioTransformType::Trim {
                start_seconds,
                end_seconds,
            } => {
                self.trim(input_temp.path(), output_path, start_seconds, end_seconds)
                    .await?;
            }
            AudioTransformType::Normalize => {
                self.normalize(input_temp.path(), output_path, options.params)
                    .await?;
            }
            AudioTransformType::ChangeBitrate { bitrate_kbps } => {
                self.change_bitrate(input_temp.path(), output_path, bitrate_kbps, options.params)
                    .await?;
            }
        }

        // Read output file
        let output_data = tokio::fs::read(output_path).await?;
        Ok(Bytes::from(output_data))
    }

    fn supported_transforms(&self) -> Vec<TransformType> {
        vec![
            TransformType::AudioTranscode,
            TransformType::AudioTrim,
            TransformType::AudioNormalize,
            TransformType::AudioBitrate,
        ]
    }
}

impl AudioTransformer {
    async fn transcode(
        &self,
        input_path: &Path,
        output_path: &Path,
        format: &str,
        params: AudioTransformParams,
    ) -> Result<()> {
        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

        // Set audio codec based on format
        let (codec, file_format) = match format {
            "mp3" => ("libmp3lame", "mp3"),
            "aac" => ("aac", "aac"),
            "wav" => ("pcm_s16le", "wav"),
            "ogg" => ("libvorbis", "ogg"),
            "flac" => ("flac", "flac"),
            _ => ("libmp3lame", "mp3"),
        };

        args.extend_from_slice(&["-acodec".to_string(), codec.to_string()]);
        args.extend_from_slice(&["-f".to_string(), file_format.to_string()]);

        // Set quality if specified
        if let Some(quality) = params.quality {
            match format {
                "mp3" => {
                    let q = match quality.as_str() {
                        "high" => "0",
                        "medium" => "4",
                        "low" => "9",
                        _ => "4",
                    };
                    args.extend_from_slice(&["-q:a".to_string(), q.to_string()]);
                }
                "aac" => {
                    let br = match quality.as_str() {
                        "high" => "256k",
                        "medium" => "192k",
                        "low" => "128k",
                        _ => "192k",
                    };
                    args.extend_from_slice(&["-b:a".to_string(), br.to_string()]);
                }
                _ => {}
            }
        }

        args.push("-y".to_string());
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

    async fn trim(
        &self,
        input_path: &Path,
        output_path: &Path,
        start: f64,
        end: Option<f64>,
    ) -> Result<()> {
        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

        // Seek to start
        args.extend_from_slice(&["-ss".to_string(), start.to_string()]);

        // Set duration if end is specified
        if let Some(end_time) = end {
            let duration = end_time - start;
            args.extend_from_slice(&["-t".to_string(), duration.to_string()]);
        }

        // Copy codec to avoid re-encoding
        args.extend_from_slice(&["-acodec".to_string(), "copy".to_string()]);
        args.push("-y".to_string());
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

    async fn normalize(
        &self,
        input_path: &Path,
        output_path: &Path,
        _params: AudioTransformParams,
    ) -> Result<()> {
        // Use loudnorm filter for audio normalization
        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

        args.extend_from_slice(&[
            "-af".to_string(),
            "loudnorm=I=-16:TP=-1.5:LRA=11".to_string(),
        ]);

        args.extend_from_slice(&["-acodec".to_string(), "aac".to_string()]);
        args.push("-y".to_string());
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
            return Err(anyhow!("FFmpeg normalize failed: {}", stderr));
        }

        Ok(())
    }

    async fn change_bitrate(
        &self,
        input_path: &Path,
        output_path: &Path,
        bitrate_kbps: u32,
        _params: AudioTransformParams,
    ) -> Result<()> {
        let mut args = vec!["-i".to_string(), input_path.to_string_lossy().to_string()];

        args.extend_from_slice(&["-b:a".to_string(), format!("{}k", bitrate_kbps)]);
        args.extend_from_slice(&["-acodec".to_string(), "aac".to_string()]);
        args.push("-y".to_string());
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
            return Err(anyhow!("FFmpeg bitrate change failed: {}", stderr));
        }

        Ok(())
    }
}
