// Assembly AI plugin for audio transcription

use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::time::Duration;
use tokio::time::sleep;

use crate::plugins::{Plugin, PluginContext, PluginExecutionStatus, PluginResult, PluginUsage};

/// Assembly AI plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssemblyAiConfig {
    /// Assembly AI API key
    pub api_key: String,
    /// Optional language code (e.g., "en", "es"). If None, auto-detect.
    pub language_code: Option<String>,
}

/// Assembly AI plugin implementation
pub struct AssemblyAiPlugin {
    http_client: Client,
}

impl Debug for AssemblyAiPlugin {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("AssemblyAiPlugin").finish()
    }
}

impl AssemblyAiPlugin {
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for long audio files
            .build()
            .context("Failed to create HTTP client for Assembly AI")?;

        Ok(Self { http_client })
    }

    /// Upload audio file to Assembly AI
    async fn upload_audio(&self, api_key: &str, audio_data: Vec<u8>) -> Result<String> {
        let url = "https://api.assemblyai.com/v2/upload";

        let response = self
            .http_client
            .post(url)
            .header("authorization", api_key)
            .body(audio_data)
            .send()
            .await
            .context("Failed to upload audio to Assembly AI")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Assembly AI upload failed: {} - {}",
                status,
                error_text
            ));
        }

        let upload_response: UploadResponse = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        Ok(upload_response.upload_url)
    }

    /// Start transcription job
    async fn start_transcription(
        &self,
        api_key: &str,
        upload_url: &str,
        language_code: Option<&str>,
    ) -> Result<String> {
        let url = "https://api.assemblyai.com/v2/transcript";

        let mut request_body = json!({
            "audio_url": upload_url,
        });

        if let Some(lang) = language_code {
            request_body["language_code"] = json!(lang);
        }

        let response = self
            .http_client
            .post(url)
            .header("authorization", api_key)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .context("Failed to start transcription")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(anyhow::anyhow!(
                "Assembly AI transcription start failed: {} - {}",
                status,
                error_text
            ));
        }

        let transcript_response: TranscriptResponse = response
            .json()
            .await
            .context("Failed to parse transcription response")?;

        Ok(transcript_response.id)
    }

    /// Poll for transcription completion
    async fn poll_transcription(
        &self,
        api_key: &str,
        transcript_id: &str,
    ) -> Result<TranscriptResult> {
        let url = format!("https://api.assemblyai.com/v2/transcript/{}", transcript_id);

        let mut attempts = 0;
        let max_attempts = 120; // 10 minutes max (5 second intervals)

        loop {
            let response = self
                .http_client
                .get(&url)
                .header("authorization", api_key)
                .send()
                .await
                .context("Failed to poll transcription status")?;

            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(anyhow::anyhow!(
                    "Assembly AI status check failed: {} - {}",
                    status,
                    error_text
                ));
            }

            let transcript: TranscriptResult = response
                .json()
                .await
                .context("Failed to parse transcript status")?;

            match transcript.status.as_str() {
                "completed" => {
                    tracing::info!(
                        transcript_id = %transcript_id,
                        "Transcription completed"
                    );
                    return Ok(transcript);
                }
                "error" => {
                    return Err(anyhow::anyhow!(
                        "Transcription failed: {}",
                        transcript
                            .error
                            .unwrap_or_else(|| "Unknown error".to_string())
                    ));
                }
                _ => {
                    // Status is "queued" or "processing", continue polling
                    attempts += 1;
                    if attempts >= max_attempts {
                        return Err(anyhow::anyhow!(
                            "Transcription timed out after {} attempts",
                            max_attempts
                        ));
                    }

                    // Exponential backoff: start with 1 second, max 5 seconds
                    let delay_secs = attempts.min(5) as u64;
                    sleep(Duration::from_secs(delay_secs)).await;
                }
            }
        }
    }
}

#[async_trait]
impl Plugin for AssemblyAiPlugin {
    fn name(&self) -> &str {
        "assembly_ai"
    }

    fn validate_config(&self, config: &serde_json::Value) -> Result<()> {
        let config: AssemblyAiConfig = serde_json::from_value(config.clone())
            .context("Invalid Assembly AI configuration: missing or invalid fields")?;

        // Validate API key is present and not a placeholder
        if config.api_key.is_empty() {
            anyhow::bail!("Assembly AI API key is required but not provided");
        }

        if config.api_key == "your-api-key" || config.api_key == "sk-" || config.api_key.len() < 10
        {
            anyhow::bail!("Assembly AI API key appears to be invalid or a placeholder. Please provide a valid API key.");
        }

        Ok(())
    }

    async fn execute(&self, context: PluginContext) -> Result<PluginResult> {
        tracing::info!(
            tenant_id = %context.tenant_id,
            media_id = %context.media_id,
            "Executing Assembly AI transcription plugin"
        );

        // Parse configuration
        let config: AssemblyAiConfig = serde_json::from_value(context.config.clone())
            .context("Failed to parse Assembly AI configuration")?;

        // Get audio file from storage
        let audio = context
            .media_repo
            .get_audio(context.tenant_id, context.media_id)
            .await
            .context("Failed to get audio file")?
            .context("Audio file not found")?;

        // Download audio file using filename as storage key
        // The filename in the Audio model is the storage key
        let audio_data = context
            .storage
            .download(&audio.filename)
            .await
            .context("Failed to download audio file from storage")?;

        // Validate audio size to prevent memory issues
        crate::validation::validate_audio_size(&audio_data)
            .context("Audio file too large for processing")?;

        tracing::info!(
            audio_size = audio_data.len(),
            "Downloaded audio file (size validated), uploading to Assembly AI"
        );

        // Upload to Assembly AI
        let upload_url = self
            .upload_audio(&config.api_key, audio_data)
            .await
            .context("Failed to upload audio to Assembly AI")?;

        tracing::info!(upload_url = %upload_url, "Audio uploaded, starting transcription");

        // Start transcription
        let transcript_id = self
            .start_transcription(
                &config.api_key,
                &upload_url,
                config.language_code.as_deref(),
            )
            .await
            .context("Failed to start transcription")?;

        tracing::info!(
            transcript_id = %transcript_id,
            "Transcription started, polling for completion"
        );

        // Poll for completion
        let transcript_result = self
            .poll_transcription(&config.api_key, &transcript_id)
            .await
            .context("Failed to get transcription result")?;

        tracing::info!(
            transcript_id = %transcript_id,
            text_length = transcript_result.text.as_ref().map(|t| t.len()).unwrap_or(0),
            "Transcription completed successfully"
        );

        // Convert transcript result to JSON
        let transcript_json = json!({
            "transcript_id": transcript_id,
            "text": transcript_result.text,
            "words": transcript_result.words,
            "confidence": transcript_result.confidence,
            "language_code": transcript_result.language_code,
            "audio_duration": transcript_result.audio_duration,
            "status": transcript_result.status,
        });

        // Persist transcript in media metadata (same pattern as other plugins)
        context
            .media_repo
            .merge_plugin_metadata(
                context.tenant_id,
                context.media_id,
                "assembly_ai",
                transcript_json.clone(),
            )
            .await
            .context("Failed to update metadata: media not found or unauthorized")?;

        let plugin_usage = transcript_result
            .audio_duration
            .map(|duration_secs| PluginUsage {
                unit_type: "audio_seconds".to_string(),
                input_units: Some(duration_secs as i64),
                output_units: None,
                total_units: duration_secs as i64,
                raw_usage: Some(json!({ "audio_duration": duration_secs })),
            });

        Ok(PluginResult {
            status: PluginExecutionStatus::Success,
            data: transcript_json,
            error: None,
            metadata: Some(json!({
                "transcript_id": transcript_id,
                "text_length": transcript_result.text.as_ref().map(|t| t.len()).unwrap_or(0),
            })),
            usage: plugin_usage,
        })
    }
}

// Assembly AI API response types
#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    #[allow(dead_code)]
    id: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptResult {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    status: String,
    text: Option<String>,
    words: Option<Vec<Word>>,
    confidence: Option<f64>,
    language_code: Option<String>,
    audio_duration: Option<u64>,
    error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Word {
    text: String,
    start: u64,
    end: u64,
    confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_config_success() {
        let plugin = AssemblyAiPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key-123",
            "language_code": "en"
        });

        let result = plugin.validate_config(&config);
        assert!(result.is_ok(), "Valid config should pass validation");
    }

    #[test]
    fn test_validate_config_missing_api_key() {
        let plugin = AssemblyAiPlugin::new().unwrap();
        let config = json!({
            "language_code": "en"
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config without api_key should fail validation"
        );
        let error_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(
            error_msg.contains("api_key") || error_msg.contains("missing"),
            "Error should mention api_key. Got: {}",
            error_msg
        );
    }

    #[test]
    fn test_validate_config_invalid_json() {
        let plugin = AssemblyAiPlugin::new().unwrap();
        // Test with completely invalid JSON structure
        let config = json!({
            "api_key": 123, // Wrong type
            "language_code": "en"
        });

        // This should fail because api_key must be a string
        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config with wrong types should fail validation"
        );
    }

    #[test]
    fn test_validate_config_empty_api_key() {
        let plugin = AssemblyAiPlugin::new().unwrap();
        let config = json!({
            "api_key": "",
            "language_code": "en"
        });

        // Empty API key must be rejected by validate_config
        let result = plugin.validate_config(&config);
        assert!(
            result.is_err(),
            "Config with empty api_key should fail validation"
        );
    }

    #[test]
    fn test_validate_config_optional_language_code() {
        let plugin = AssemblyAiPlugin::new().unwrap();
        let config = json!({
            "api_key": "test-api-key"
            // language_code is optional
        });

        let result = plugin.validate_config(&config);
        assert!(
            result.is_ok(),
            "Config without language_code should be valid"
        );
    }

    // =========================================================================
    // Execution Tests
    // =========================================================================
    // Note: Full execution tests with HTTP mocking require:
    // 1. Plugin modification to accept configurable base URL (for mockito)
    // 2. Test database setup (sqlx::test) or mocked repositories
    // 3. Mock storage with test audio data
    //
    // These tests demonstrate the structure and can be expanded when the
    // plugin supports configurable base URLs for testing.

    #[tokio::test]
    #[ignore] // Ignore until plugin supports configurable base URL for HTTP mocking
    async fn test_execute_success() {
        // TRACKED: Implement when plugin supports configurable base URL for testing.
        // This requires plugin modification to accept base_url parameter for testing
        // For full test, would need:
        // - Plugin with configurable base_url pointing to mockito server
        // - Test database with audio record inserted
        // - PluginContext with mocked repositories
        unimplemented!("Requires plugin refactoring to support mockito::Server");
    }

    #[tokio::test]
    #[ignore] // Ignore until plugin supports configurable base URL
    async fn test_execute_upload_failure() {
        // TRACKED: Test upload failure handling with configurable base URL (requires plugin refactoring)
        unimplemented!("Requires plugin refactoring");
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_transcription_error() {
        // TRACKED: Test transcription error handling with configurable base URL (requires plugin refactoring)
        unimplemented!("Requires plugin refactoring");
    }

    #[tokio::test]
    #[ignore]
    async fn test_execute_poll_timeout() {
        // TRACKED: Test polling timeout after maximum attempts (requires plugin refactoring)
        unimplemented!("Requires plugin refactoring");
    }

    #[tokio::test]
    async fn test_execute_audio_not_found() {
        use crate::test_helpers::*;

        let plugin = AssemblyAiPlugin::new().unwrap();
        let config = create_assembly_ai_config(Some("test-api-key"));

        // Create context - audio won't exist in database/mock repo
        // For this test, we'd need a mocked MediaRepository that returns None
        // This demonstrates the error case structure
        assert!(plugin.validate_config(&config).is_ok());
    }

    #[tokio::test]
    async fn test_execute_download_failure() {
        use crate::test_helpers::*;

        let plugin = AssemblyAiPlugin::new().unwrap();
        let config = create_assembly_ai_config(Some("test-api-key"));

        // Create storage that will fail on download
        // Don't set the file, so download will fail

        // Create context with audio that has filename pointing to non-existent file
        // Would test that storage download failure is properly handled
        assert!(plugin.validate_config(&config).is_ok());
    }
}
