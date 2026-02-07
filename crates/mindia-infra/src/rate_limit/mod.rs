//! Rate limiting service
//!
//! This module provides rate limiting functionality using token bucket algorithm.

pub use limiter::RateLimiter;

mod limiter;
