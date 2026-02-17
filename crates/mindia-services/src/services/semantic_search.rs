//! Semantic search provider abstraction.
//!
//! Cloud providers (e.g. Anthropic/Claude) for vision, document summarization, and embeddings.

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;

/// Required embedding dimension for pgvector compatibility.
pub const EMBEDDING_DIM: usize = 768;

/// Provider for semantic search: vision, document summarization, and embeddings.
/// Implemented by Anthropic/Claude (cloud).
#[async_trait]
pub trait SemanticSearchProvider: Send + Sync {
    /// Human-readable model name stored with embeddings (e.g. "nomic-embed-text", "embed-v3").
    fn embedding_model_name(&self) -> &str;

    /// Check if the provider is available and responsive.
    async fn health_check(&self) -> Result<bool>;

    /// Generate an embedding vector from text. Must return exactly [EMBEDDING_DIM] dimensions.
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Describe an image for indexing (vision).
    /// `content_type` - Optional MIME type (e.g. "image/jpeg", "image/png"); defaults to "image/jpeg" if None.
    async fn describe_image(&self, image_data: Bytes, content_type: Option<&str>)
        -> Result<String>;

    /// Describe a video frame for indexing (vision).
    /// `content_type` - Optional MIME type for the frame; defaults to "image/jpeg" if None.
    async fn describe_video_frame(
        &self,
        frame_data: Bytes,
        content_type: Option<&str>,
    ) -> Result<String>;

    /// Summarize document text for indexing.
    async fn summarize_document(&self, text: &str) -> Result<String>;
}

/// Normalize embedding to [EMBEDDING_DIM] (truncate or zero-pad). Used by cloud providers
/// that may return different dimensions.
pub fn normalize_embedding_dim(mut vec: Vec<f32>) -> Vec<f32> {
    if vec.len() == EMBEDDING_DIM {
        return vec;
    }
    if vec.len() > EMBEDDING_DIM {
        vec.truncate(EMBEDDING_DIM);
        return vec;
    }
    vec.resize(EMBEDDING_DIM, 0.0);
    vec
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_embedding_dim_exact() {
        let input: Vec<f32> = (0..EMBEDDING_DIM).map(|i| i as f32).collect();
        let result = normalize_embedding_dim(input.clone());
        assert_eq!(result.len(), EMBEDDING_DIM);
        assert_eq!(result, input);
    }

    #[test]
    fn normalize_embedding_dim_truncate() {
        let input: Vec<f32> = (0..EMBEDDING_DIM + 100).map(|i| i as f32).collect();
        let result = normalize_embedding_dim(input);
        assert_eq!(result.len(), EMBEDDING_DIM);
        assert_eq!(result[EMBEDDING_DIM - 1], (EMBEDDING_DIM - 1) as f32);
    }

    #[test]
    fn normalize_embedding_dim_pad() {
        let input: Vec<f32> = vec![1.0, 2.0, 3.0];
        let result = normalize_embedding_dim(input);
        assert_eq!(result.len(), EMBEDDING_DIM);
        assert_eq!(result[0], 1.0);
        assert_eq!(result[1], 2.0);
        assert_eq!(result[2], 3.0);
        for i in 3..EMBEDDING_DIM {
            assert_eq!(result[i], 0.0, "padded element at {} should be 0", i);
        }
    }

    #[test]
    fn normalize_embedding_dim_empty() {
        let input: Vec<f32> = vec![];
        let result = normalize_embedding_dim(input);
        assert_eq!(result.len(), EMBEDDING_DIM);
        assert!(result.iter().all(|&x| x == 0.0));
    }
}
