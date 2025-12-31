//! FFmpegService - video transcoding and thumbnail generation.

use crate::traits::MediaTransformer;
use crate::video::processor::VideoProcessor;
use crate::video::transformer::VideoTransformer;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HLSVariant {
    pub name: String,
    pub resolution: String,
    pub bitrate: u32,
    pub width: u32,
    pub height: u32,
    pub playlist_path: String,
}

pub struct FFmpegService {
    ffmpeg_path: String,
    segment_duration: u64,
    video_processor: VideoProcessor,
}

impl FFmpegService {
    pub fn new(ffmpeg_path: String, segment_duration: u64) -> Result<Self> {
        let video_processor = VideoProcessor::new("ffprobe".to_string())
            .context("Failed to create VideoProcessor")?;

        Ok(Self {
            ffmpeg_path,
            segment_duration,
            video_processor,
        })
    }

    /// Probe video file to extract metadata (delegates to VideoProcessor)
    pub async fn probe_video(&self, video_path: &Path) -> Result<crate::metadata::VideoMetadata> {
        self.video_processor
            .extract_metadata_from_path(video_path)
            .await
    }

    /// Generate HLS variant at specific resolution and bitrate
    #[tracing::instrument(skip(self, input_path, output_dir))]
    #[allow(clippy::too_many_arguments)]
    pub async fn generate_hls_variant(
        &self,
        input_path: &Path,
        output_dir: &Path,
        variant_name: &str,
        width: u32,
        height: u32,
        bitrate_kbps: u32,
        hw_encoder: Option<&str>,
    ) -> Result<HLSVariant> {
        let variant_dir = output_dir.join(variant_name);
        tokio::fs::create_dir_all(&variant_dir).await?;

        let playlist_path = variant_dir.join("index.m3u8");
        let segment_pattern = variant_dir.join("segment_%03d.ts");

        let encoder = hw_encoder.unwrap_or("libx264");

        let mut args = vec![
            "-i".to_string(),
            input_path.to_string_lossy().to_string(),
            "-c:v".to_string(),
            encoder.to_string(),
        ];

        match encoder {
            "libx264" => {
                args.extend_from_slice(&[
                    "-preset".to_string(),
                    "fast".to_string(),
                    "-profile:v".to_string(),
                    "main".to_string(),
                ]);
            }
            "h264_nvenc" | "h264_qsv" => {
                args.extend_from_slice(&["-preset".to_string(), "fast".to_string()]);
            }
            _ => {}
        }

        args.extend_from_slice(&[
            "-vf".to_string(),
            format!("scale={}:{}", width, height),
            "-b:v".to_string(),
            format!("{}k", bitrate_kbps),
            "-maxrate".to_string(),
            format!("{}k", (bitrate_kbps as f32 * 1.2) as u32),
            "-bufsize".to_string(),
            format!("{}k", bitrate_kbps * 2),
            "-c:a".to_string(),
            "aac".to_string(),
            "-b:a".to_string(),
            "128k".to_string(),
            "-ac".to_string(),
            "2".to_string(),
            "-ar".to_string(),
            "48000".to_string(),
            "-f".to_string(),
            "hls".to_string(),
            "-hls_time".to_string(),
            self.segment_duration.to_string(),
            "-hls_playlist_type".to_string(),
            "vod".to_string(),
            "-hls_segment_filename".to_string(),
            segment_pattern.to_string_lossy().to_string(),
            playlist_path.to_string_lossy().to_string(),
        ]);

        let output = Command::new(&self.ffmpeg_path)
            .args(&args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
            .await
            .context("Failed to execute ffmpeg")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("FFmpeg failed: {}", stderr));
        }

        Ok(HLSVariant {
            name: variant_name.to_string(),
            resolution: format!("{}x{}", width, height),
            bitrate: bitrate_kbps,
            width,
            height,
            playlist_path: format!("{}/index.m3u8", variant_name),
        })
    }

    /// Generate all HLS variants based on source resolution
    pub async fn generate_all_variants(
        &self,
        input_path: &Path,
        output_dir: &Path,
        _source_width: u32,
        source_height: u32,
        requested_variants: &[String],
    ) -> Result<Vec<HLSVariant>> {
        let mut variants = Vec::new();
        let mut tasks = Vec::new();

        let variant_configs = [
            ("360p", 640, 360, 800),
            ("480p", 854, 480, 1400),
            ("720p", 1280, 720, 2800),
            ("1080p", 1920, 1080, 5000),
        ];

        for (name, width, height, bitrate) in variant_configs {
            if requested_variants.contains(&name.to_string()) && source_height >= height {
                let input = input_path.to_path_buf();
                let output = output_dir.to_path_buf();
                let service = self.clone();
                let variant_name = name.to_string();

                let task = tokio::spawn(async move {
                    service
                        .generate_hls_variant(
                            &input,
                            &output,
                            &variant_name,
                            width,
                            height,
                            bitrate,
                            None,
                        )
                        .await
                });

                tasks.push(task);
            }
        }

        for task in tasks {
            match task.await {
                Ok(Ok(variant)) => variants.push(variant),
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(anyhow!("Task failed: {}", e)),
            }
        }

        if variants.is_empty() {
            return Err(anyhow!("No variants were generated"));
        }

        Ok(variants)
    }

    /// Create HLS master playlist
    pub fn create_master_playlist(&self, variants: &[HLSVariant]) -> Result<String> {
        let mut playlist = String::from("#EXTM3U\n#EXT-X-VERSION:3\n\n");

        for variant in variants {
            playlist.push_str(&format!(
                "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}\n{}\n\n",
                variant.bitrate * 1000,
                variant.resolution,
                variant.playlist_path
            ));
        }

        Ok(playlist)
    }

    /// Extract a single frame from video at specified timestamp
    pub async fn extract_frame(
        &self,
        video_path: &Path,
        output_path: &Path,
        timestamp_seconds: f64,
    ) -> Result<()> {
        let transformer = VideoTransformer::new(self.ffmpeg_path.clone())?;
        let options = crate::video::transformer::VideoTransformOptions {
            transform_type: crate::video::transformer::VideoTransformType::ExtractThumbnail {
                timestamp_seconds,
            },
            params: crate::video::transformer::VideoTransformParams {
                bitrate_kbps: None,
                quality: None,
            },
        };

        // Read video data
        let video_data = tokio::fs::read(video_path).await?;
        let thumbnail_data = transformer.transform(&video_data, options).await?;
        tokio::fs::write(output_path, thumbnail_data).await?;

        Ok(())
    }
}

impl Clone for FFmpegService {
    fn clone(&self) -> Self {
        Self {
            ffmpeg_path: self.ffmpeg_path.clone(),
            segment_duration: self.segment_duration,
            video_processor: VideoProcessor::new("ffprobe".to_string())
                .expect("Failed to clone VideoProcessor"),
        }
    }
}
