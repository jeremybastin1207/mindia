use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use mindia_core::models::TaskType;

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let tokens_to_add = elapsed * self.refill_rate;

        self.tokens = (self.tokens + tokens_to_add).min(self.capacity);
        self.last_refill = now;
    }

    fn try_acquire(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn time_until_next_token(&self) -> Duration {
        if self.tokens >= 1.0 {
            Duration::from_secs(0)
        } else {
            let tokens_needed = 1.0 - self.tokens;
            let seconds = tokens_needed / self.refill_rate;
            Duration::from_secs_f64(seconds.max(0.0))
        }
    }
}

/// Sharded rate limiter to reduce lock contention under high task submission rates.
///
/// Uses multiple shards (separate HashMaps) so that different task types typically
/// lock different shards. Configurable via `TASK_RATE_LIMITER_SHARD_COUNT` (default: 16).
#[derive(Clone)]
pub struct RateLimiter {
    shards: Vec<Arc<Mutex<HashMap<TaskType, TokenBucket>>>>,
    shard_count: usize,
    default_rate: f64,
    video_rate: f64,
    embedding_rate: f64,
}

impl RateLimiter {
    fn shard_index(&self, task_type: &TaskType) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        task_type.hash(&mut hasher);
        (hasher.finish() as usize) % self.shard_count
    }

    fn initial_bucket(&self, task_type: &TaskType) -> TokenBucket {
        match task_type {
            TaskType::VideoTranscode => TokenBucket::new(self.video_rate * 2.0, self.video_rate),
            TaskType::GenerateEmbedding => {
                TokenBucket::new(self.embedding_rate * 2.0, self.embedding_rate)
            }
            _ => TokenBucket::new(self.default_rate * 2.0, self.default_rate),
        }
    }

    /// Create a new rate limiter with per-task-type configurations (default 16 shards).
    pub fn new(video_rate: f64, embedding_rate: f64) -> Self {
        Self::with_shards(video_rate, embedding_rate, 16)
    }

    /// Create a rate limiter with custom shard count for tuning under high load.
    pub fn with_shards(video_rate: f64, embedding_rate: f64, shard_count: usize) -> Self {
        let shards = (0..shard_count)
            .map(|_| Arc::new(Mutex::new(HashMap::new())))
            .collect();
        Self {
            shards,
            shard_count,
            default_rate: 1.0,
            video_rate,
            embedding_rate,
        }
    }

    /// Acquire a token for the given task type, blocking until available
    #[tracing::instrument(skip(self))]
    pub async fn acquire(&self, task_type: &TaskType) {
        loop {
            let wait_duration = {
                let shard_index = self.shard_index(task_type);
                let shard = &self.shards[shard_index];
                let mut buckets = shard.lock().await;
                let bucket = buckets
                    .entry(task_type.clone())
                    .or_insert_with(|| self.initial_bucket(task_type));

                if bucket.try_acquire() {
                    tracing::trace!(
                        task_type = %task_type,
                        tokens_remaining = bucket.tokens,
                        "Rate limit token acquired"
                    );
                    return; // Token acquired successfully
                }

                bucket.time_until_next_token()
            };

            if wait_duration > Duration::from_secs(0) {
                tracing::debug!(
                    task_type = %task_type,
                    wait_ms = wait_duration.as_millis(),
                    "Rate limit reached, waiting for token"
                );
                tokio::time::sleep(wait_duration).await;
            }
        }
    }

    /// Try to acquire a token without blocking
    #[tracing::instrument(skip(self))]
    pub async fn try_acquire(&self, task_type: &TaskType) -> bool {
        let shard_index = self.shard_index(task_type);
        let shard = &self.shards[shard_index];
        let mut buckets = shard.lock().await;
        let bucket = buckets
            .entry(task_type.clone())
            .or_insert_with(|| self.initial_bucket(task_type));

        let acquired = bucket.try_acquire();

        if acquired {
            tracing::trace!(
                task_type = %task_type,
                tokens_remaining = bucket.tokens,
                "Rate limit token acquired (non-blocking)"
            );
        } else {
            tracing::trace!(
                task_type = %task_type,
                tokens_remaining = bucket.tokens,
                "Rate limit token not available"
            );
        }

        acquired
    }

    /// Get the current number of available tokens for a task type
    #[tracing::instrument(skip(self))]
    pub async fn available_tokens(&self, task_type: &TaskType) -> f64 {
        let shard_index = self.shard_index(task_type);
        let shard = &self.shards[shard_index];
        let mut buckets = shard.lock().await;
        let bucket = buckets
            .entry(task_type.clone())
            .or_insert_with(|| self.initial_bucket(task_type));

        bucket.refill();
        bucket.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(2.0, 5.0); // 2 video/sec, 5 embedding/sec

        // Should acquire immediately
        limiter.acquire(&TaskType::VideoTranscode).await;
        limiter.acquire(&TaskType::VideoTranscode).await;

        // Check available tokens (should be less than capacity)
        let tokens = limiter.available_tokens(&TaskType::VideoTranscode).await;
        assert!(tokens < 4.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new(10.0, 10.0);

        // Drain some tokens
        for _ in 0..5 {
            limiter.acquire(&TaskType::GenerateEmbedding).await;
        }

        let tokens_before = limiter.available_tokens(&TaskType::GenerateEmbedding).await;

        // Wait for refill
        tokio::time::sleep(Duration::from_millis(500)).await;

        let tokens_after = limiter.available_tokens(&TaskType::GenerateEmbedding).await;

        // Should have refilled some tokens
        assert!(tokens_after > tokens_before);
    }

    #[tokio::test]
    async fn test_try_acquire_non_blocking() {
        let limiter = RateLimiter::new(1.0, 1.0);

        // Should succeed
        assert!(limiter.try_acquire(&TaskType::VideoTranscode).await);

        // Drain the bucket
        while limiter.try_acquire(&TaskType::VideoTranscode).await {}

        // Should fail without blocking
        assert!(!limiter.try_acquire(&TaskType::VideoTranscode).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_single_shard() {
        let limiter = RateLimiter::with_shards(2.0, 5.0, 1);

        // Single shard: all task types share the same lock
        limiter.acquire(&TaskType::VideoTranscode).await;
        limiter.acquire(&TaskType::GenerateEmbedding).await;

        let video_tokens = limiter.available_tokens(&TaskType::VideoTranscode).await;
        let embedding_tokens = limiter.available_tokens(&TaskType::GenerateEmbedding).await;

        assert!(video_tokens < 4.0);
        assert!(embedding_tokens < 10.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_default_task_type_bucket() {
        let limiter = RateLimiter::new(1.0, 1.0);

        // PluginExecution and other non-video/embedding types use default rate (1.0/sec, capacity 2.0)
        let task_type = TaskType::PluginExecution;
        assert!(limiter.try_acquire(&task_type).await);
        assert!(limiter.try_acquire(&task_type).await);
        assert!(!limiter.try_acquire(&task_type).await);

        let tokens = limiter.available_tokens(&task_type).await;
        assert!(tokens < 2.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_video_vs_embedding_capacities() {
        let limiter = RateLimiter::new(2.0, 5.0);

        // Video: capacity 4.0 (2 * 2), refill 2/sec
        let video_tokens = limiter.available_tokens(&TaskType::VideoTranscode).await;
        assert_eq!(video_tokens, 4.0);

        // Embedding: capacity 10.0 (2 * 5), refill 5/sec
        let embedding_tokens = limiter.available_tokens(&TaskType::GenerateEmbedding).await;
        assert_eq!(embedding_tokens, 10.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_drain_then_refill() {
        let limiter = RateLimiter::new(10.0, 10.0);

        for _ in 0..20 {
            limiter.acquire(&TaskType::GenerateEmbedding).await;
        }

        let tokens_after_drain = limiter.available_tokens(&TaskType::GenerateEmbedding).await;
        assert!(tokens_after_drain < 1.0);

        tokio::time::sleep(Duration::from_millis(200)).await;
        let tokens_after_refill = limiter.available_tokens(&TaskType::GenerateEmbedding).await;
        assert!(tokens_after_refill > tokens_after_drain);
    }
}
