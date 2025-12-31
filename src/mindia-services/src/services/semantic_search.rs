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
    async fn describe_image(&self, image_data: Bytes) -> Result<String>;

    /// Describe a video frame for indexing (vision).
    async fn describe_video_frame(&self, frame_data: Bytes) -> Result<String>;

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
