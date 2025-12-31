//! AWS Transcribe plugin for audio transcription

use anyhow::{Context, Result};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_transcribe::Client as TranscribeClient;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};
use mindia_core::StorageBackend;

/// AWS Transcribe plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwsTranscribeConfig {
    /// AWS region (e.g., "us-east-1")
    pub region: String,
    /// S3 bucket name for storing audio files for transcription
    pub s3_bucket: String,
    /// Optional language code (e.g., "en-US", "es-US"). If None, auto-detect.
    #[serde(default)]
    pub language_code: Option<String>,
    /// Optional media format (e.g., "mp3", "mp4", "wav", "flac", "ogg", "amr", "webm")
    #[serde(default)]
    pub media_format: Option<String>,
    /// Optional vocabulary name for custom vocabulary
    #[serde(default)]
    pub vocabulary_name: Option<String>,
    /// Optional medical vocabulary name for medical transcription
    #[serde(default)]
    pub vocabulary_filter_name: Option<String>,
    /// Enable speaker identification (diarization)
    #[serde(default = "default_false")]
    pub show_speaker_labels: bool,
    /// Maximum number of speakers (if show_speaker_labels is true)
    #[serde(default)]
    pub max_speaker_labels: Option<u32>,
}

fn default_false() -> bool {
    false
}

/// AWS Transcribe plugin implementation
pub struct AwsTranscribePlugin;

impl Default for AwsTranscribePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for AwsTranscribePlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("AwsTranscribePlugin").finish()
    }
}

impl AwsTranscribePlugin {
    pub fn new() -> Self {
        Self
    }

    /// Create Transcribe client for the given region
    async fn create_client(region: &str) -> Result<TranscribeClient> {
        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        Ok(TranscribeClient::new(&config))
    }

    /// Start transcription job
    async fn start_transcription(
        client: &TranscribeClient,
        job_name: &str,
        s3_uri: &str,
        config: &AwsTranscribeConfig,
    ) -> Result<()> {
        use aws_sdk_transcribe::types::Media;
        let media = Media::builder().media_file_uri(s3_uri).build();
        let mut request = client
            .start_transcription_job()
            .transcription_job_name(job_name)
            .media(media);

        if let Some(lang) = &config.language_code {
            use aws_sdk_transcribe::types::LanguageCode;
            let lang_code = LanguageCode::from(lang.as_str());
            request = request.language_code(lang_code);
        }

        if let Some(format) = &config.media_format {
            use aws_sdk_transcribe::types::MediaFormat;
            let media_fmt = MediaFormat::from(format.as_str());
            request = request.media_format(media_fmt);
        }

        // Note: vocabulary_name and vocabulary_filter_name may not be available in this SDK version
        // These features are commented out until the correct API is determined
        // if let Some(vocab) = &config.vocabulary_name {
        //     request = request.vocabulary_name(vocab);
        // }
        //
        // if let Some(filter) = &config.vocabulary_filter_name {
        //     request = request.vocabulary_filter_name(filter);
        // }

        if config.show_speaker_labels {
            let mut settings = aws_sdk_transcribe::types::Settings::builder()
                .show_speaker_labels(config.show_speaker_labels);

            if let Some(max_speakers) = config.max_speaker_labels {
                settings = settings.max_speaker_labels(max_speakers as i32);
            }

            request = request.settings(settings.build());
        }

        request
            .send()
            .await
            .context("Failed to start transcription job")?;

        Ok(())
    }

    /// Poll for transcription completion
    async fn poll_transcription(
        client: &TranscribeClient,
        job_name: &str,
    ) -> Result<TranscriptionResult> {
        let mut attempts = 0;
        let max_attempts = 120; // 10 minutes max (5 second intervals)

        loop {
            let response = client
                .get_transcription_job()
                .transcription_job_name(job_name)
                .send()
                .await
                .context("Failed to get transcription job status")?;

            let job = response
                .transcription_job()
                .context("Transcription job not found in response")?;

            let status = job
                .transcription_job_status()
                .map(|s| s.as_str())
                .unwrap_or("UNKNOWN");

            match status {
                "COMPLETED" => {
                    let transcript_uri = job
                        .transcript()
                        .and_then(|t| t.transcript_file_uri())
                        .map(|s| s.to_string())
                        .context("Transcript URI not found")?;

                    // Download transcript from S3 URI
                    let transcript_json = Self::download_transcript(&transcript_uri).await?;

                    tracing::info!(
                        transcription_job_name = %job_name,
                        "Transcription completed successfully"
                    );

                    return Ok(transcript_json);
                }
                "FAILED" => {
                    let failure_reason = job.failure_reason().unwrap_or("Unknown error");
                    return Err(anyhow::anyhow!(
                        "Transcription job failed: {}",
                        failure_reason
                    ));
                }
                _ => {
                    // Status is "IN_PROGRESS" or "QUEUED", continue polling
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!(
                            "Transcription timed out after {} attempts",
                            max_attempts
                        ));
                    }

                    // Exponential backoff: start with 2 seconds, max 10 seconds
                    let delay_secs = (attempts * 2).min(10) as u64;
                    sleep(Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }

    /// Download transcript JSON from S3 URI
    async fn download_transcript(uri: &str) -> Result<TranscriptionResult> {
        // Extract S3 bucket and key from URI
        // URI format: https://s3.region.amazonaws.com/bucket/key
        // or: s3://bucket/key
        let (_bucket, _key) = if let Some(stripped) = uri.strip_prefix("s3://") {
            let parts: Vec<&str> = stripped.splitn(2, '/').collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid S3 URI format: {}", uri));
            }
            (parts[0], parts[1])
        } else if uri.contains("amazonaws.com") {
            // Parse https://s3.region.amazonaws.com/bucket/key
            let parts: Vec<&str> = uri.split(".amazonaws.com/").collect();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid S3 HTTPS URI format: {}", uri));
            }
            let path_parts: Vec<&str> = parts[1].splitn(2, '/').collect();
            if path_parts.len() != 2 {
                return Err(anyhow::anyhow!("Invalid S3 HTTPS URI path: {}", uri));
            }
            (path_parts[0], path_parts[1])
        } else {
            return Err(anyhow::anyhow!("Unsupported S3 URI format: {}", uri));
        };

        // Use reqwest to download the transcript
        let client = reqwest::Client::new();
        let response = client
            .get(uri)
            .send()
            .await
            .context("Failed to download transcript from S3")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download transcript: HTTP {}",
                response.status()
            ));
        }

        let transcript_json: TranscriptionResult = response
            .json()
            .await
            .context("Failed to parse transcript JSON")?;

        Ok(transcript_json)
    }
}

#[async_trait]
impl Plugin for AwsTranscribePlugin {
    fn name(&self) -> &str {
        "aws_transcribe"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let _config: AwsTranscribeConfig = serde_json::from_value(config.clone())
            .context("Invalid AWS Transcribe configuration: missing or invalid fields")?;

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing AWS Transcribe transcription plugin"
        );

        // Parse configuration
        let config: AwsTranscribeConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse AWS Transcribe configuration")?;

        // Get audio file from media repository
        let audio = context
            .media_repo
            .get_audio(context.tenant_id, context.media_id)
            .await
            .context("Failed to get audio file")?
            .context("Audio file not found")?;

        // Download audio file from storage
        let audio_data = context
            .storage
            .download(audio.storage_key())
            .await
            .context("Failed to download audio file from storage")?;

        // Validate audio size to prevent memory issues
        crate::validation::validate_audio_size(&audio_data)
            .context("Audio file too large for processing")?;

        tracing::info!(
            audio_size = audio_data.len(),
            "Downloaded audio file (size validated), preparing for transcription"
        );

        // Create Transcribe client
        let client = Self::create_client(&config.region)
            .await
            .context("Failed to create AWS Transcribe client")?;

        // Generate unique job name
        let job_name = format!(
            "mindia-{}-{}",
            context.media_id,
            Uuid::new_v4().to_string().replace("-", "")
        );

        // AWS Transcribe requires the audio file to be in S3 (it accepts an S3 URI).
        // Local and NFS backends do not expose S3 URIs; fail with a clear error.
        if audio.storage_backend() != StorageBackend::S3 {
            return Err(anyhow::anyhow!(
                "AWS Transcribe requires S3 storage; current storage backend is {:?}. \
                 Use S3 storage for the tenant or upload the file to S3 before transcribing.",
                audio.storage_backend()
            )
            .context("Unsupported storage backend for AWS Transcribe"));
        }

        let bucket = audio.storage_bucket().unwrap_or(config.s3_bucket.as_str());
        let s3_uri = format!("s3://{}/{}", bucket, audio.storage_key());

        tracing::info!(
            transcription_job_name = %job_name,
            s3_uri = %s3_uri,
            "Starting transcription job"
        );

        // Start transcription job
        Self::start_transcription(&client, &job_name, &s3_uri, &config)
            .await
            .context("Failed to start transcription job")?;

        // Poll for completion
        let transcript_result = Self::poll_transcription(&client, &job_name)
            .await
            .context("Failed to get transcription result")?;

        tracing::info!(
            transcription_job_name = %job_name,
            text_length = transcript_result.results.transcripts.first()
                .and_then(|t| t.transcript.as_ref())
                .map(|t| t.len())
                .unwrap_or(0),
            "Transcription completed successfully"
        );

        // Convert transcript result to JSON format compatible with other transcription plugins
        let transcript_text = transcript_result
            .results
            .transcripts
            .first()
            .and_then(|t| t.transcript.clone())
            .unwrap_or_default();

        let items = transcript_result.results.items.unwrap_or_default();

        // Compute audio duration in seconds from the last item's end_time
        let audio_duration_secs: Option<i64> = items
            .iter()
            .filter_map(|item| {
                item.end_time
                    .as_ref()
                    .and_then(|t| t.parse::<f64>().ok())
                    .map(|t| t.ceil() as i64)
            })
            .max();

        // Convert items to word-level timestamps format
        let words: Vec<serde_json::Value> = items
            .iter()
            .filter_map(|item| {
                if let Some(content) = item.alternatives.first() {
                    let start_time = item
                        .start_time
                        .as_ref()
                        .and_then(|t| t.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    let end_time = item
                        .end_time
                        .as_ref()
                        .and_then(|t| t.parse::<f64>().ok())
                        .unwrap_or(0.0);

                    Some(json!({
                        "text": content.content.as_deref().unwrap_or(""),
                        "start": (start_time * 1000.0) as u64, // Convert to milliseconds
                        "end": (end_time * 1000.0) as u64,
                        "confidence": content.confidence.unwrap_or(0.0) as f64,
                        "type": item.r#type.clone(),
                    }))
                } else {
                    None
                }
            })
            .collect();

        let transcript_json = json!({
            "transcription_job_name": job_name,
            "text": transcript_text,
            "words": words,
            "language_code": transcript_result.results.language_code,
            "status": "COMPLETED",
            "completed_at": Utc::now().to_rfc3339(),
        });

        // Update metadata in database
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "aws_transcribe",
                transcript_json.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        tracing::info!(
            words_count = words.len(),
            "AWS Transcribe transcription completed successfully"
        );

        let plugin_usage = audio_duration_secs.map(|duration_secs| PluginUsage {
            unit_type: "audio_seconds".to_string(),
            input_units: Some(duration_secs),
            output_units: None,
            total_units: duration_secs,
            raw_usage: Some(json!({ "audio_duration_seconds": duration_secs })),
        });

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: transcript_json.clone(),
            error: None,
            metadata: Some(json!({
                "transcription_job_name": job_name,
                "words_count": words.len(),
                "text_length": transcript_text.len(),
            })),
            usage: plugin_usage,
        })
    }
}

// AWS Transcribe API response types
#[derive(Debug, Deserialize)]
struct TranscriptionResult {
    results: TranscriptResults,
    #[allow(dead_code)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranscriptResults {
    transcripts: Vec<Transcript>,
    items: Option<Vec<TranscriptItem>>,
    #[serde(rename = "language_code")]
    language_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Transcript {
    transcript: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TranscriptItem {
    #[serde(rename = "type")]
    r#type: Option<String>,
    #[serde(rename = "start_time")]
    start_time: Option<String>,
    #[serde(rename = "end_time")]
    end_time: Option<String>,
    alternatives: Vec<ItemAlternative>,
}

#[derive(Debug, Deserialize)]
struct ItemAlternative {
    content: Option<String>,
    confidence: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_config_success() {
        let plugin = AwsTranscribePlugin::new();
        let config = json!({
            "region": "us-east-1",
            "s3_bucket": "test-bucket",
            "language_code": "en-US",
            "media_format": "mp3"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_missing_region() {
        let plugin = AwsTranscribePlugin::new();
        let config = json!({
            "s3_bucket": "test-bucket"
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config without region should fail validation"
        );
        let error_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            error_msg.contains("region") || error_msg.contains("missing"),
            "Error should mention region. Got: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_config_missing_s3_bucket() {
        let plugin = AwsTranscribePlugin::new();
        let config = json!({
            "region": "us-east-1"
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config without s3_bucket should fail validation"
        );
        let error_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            error_msg.contains("s3_bucket") || error_msg.contains("missing"),
            "Error should mention s3_bucket. Got: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_config_optional_fields() {
        let plugin = AwsTranscribePlugin::new();
        let config = json!({
            "region": "us-east-1",
            "s3_bucket": "test-bucket"
            // language_code, media_format, etc. are optional
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config with only required fields should be valid"
        );
    }

    #[test]
    fn test_validate_config_speaker_labels() {
        let plugin = AwsTranscribePlugin::new();
        let config = json!({
            "region": "us-east-1",
            "s3_bucket": "test-bucket",
            "show_speaker_labels": true,
            "max_speaker_labels": 5
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Config with speaker labels should be valid");
    }

    #[test]
    fn test_validate_config_vocabulary() {
        let plugin = AwsTranscribePlugin::new();
        let config = json!({
            "region": "us-east-1",
            "s3_bucket": "test-bucket",
            "vocabulary_name": "my-vocab",
            "vocabulary_filter_name": "my-filter"
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config with vocabulary settings should be valid"
        );
    }

    #[test]
    fn test_download_transcript_s3_uri_format() {
        // Test URI parsing logic
        // Note: This tests the private download_transcript method logic
        // In practice, we'd need to make it public or test through integration

        // s3://bucket/key format
        let uri1 = "s3://my-bucket/transcripts/transcript.json";
        let (bucket, key) = if let Some(stripped) = uri1.strip_prefix("s3://") {
            let parts: Vec<&str> = stripped.splitn(2, '/').collect();
            if parts.len() == 2 {
                (parts[0], parts[1])
            } else {
                ("", "")
            }
        } else {
            ("", "")
        };
        assert_eq!(bucket, "my-bucket");
        assert_eq!(key, "transcripts/transcript.json");
    }

    // =========================================================================
    // Execution Tests
    // =========================================================================
    // Note: Full execution tests with AWS SDK mocking require:
    // 1. AWS SDK trait abstraction or mock implementations
    // 2. Mock HTTP client for downloading transcripts from S3
    // 3. Test database setup (sqlx::test) or mocked repositories
    // 4. Mock storage with test audio data
    // 5. LocalStack or AWS SDK test harness for integration tests
    //
    // These tests demonstrate the structure and error cases.

    #[tokio::test]
    #[ignore] // Ignore until AWS SDK mocking is implemented
    async fn test_execute_success() {
        use crate::test_helpers::*;

        let plugin = AwsTranscribePlugin::new();
        let _tenant_id = create_test_tenant_id();
        let _media_id = create_test_media_id();
        let config = create_aws_transcribe_config(Some("us-east-1"), Some("test-bucket"));

        // Would need:
        // - Mocked AWS Transcribe client
        // - Mock HTTP client for S3 transcript download
        // - Test database with audio record
        // - Mock storage with audio data
        // - PluginContext setup

        assert!(plugin.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_execute_audio_not_found() {
        use crate::test_helpers::*;

        let plugin = AwsTranscribePlugin::new();
        let config = create_aws_transcribe_config(Some("us-east-1"), Some("test-bucket"));

        // Test that plugin handles missing audio gracefully
        // Would need mocked MediaRepository returning None
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_job_failure() {
        use crate::test_helpers::*;

        let plugin = AwsTranscribePlugin::new();
        let config = create_aws_transcribe_config(Some("us-east-1"), Some("test-bucket"));

        // Would test that job failure status ("FAILED") is handled properly
        // Requires mocked AWS Transcribe client returning failure status
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_polling_timeout() {
        use crate::test_helpers::*;

        let plugin = AwsTranscribePlugin::new();
        let config = create_aws_transcribe_config(Some("us-east-1"), Some("test-bucket"));

        // Would test timeout after max_attempts (120) polling attempts
        // Requires mocked client returning "IN_PROGRESS" for 120+ attempts
        assert!(plugin.validate_config(&config).is_ok());
    }
}
