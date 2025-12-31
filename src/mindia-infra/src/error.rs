//! HTTP error response conversion
//!
//! This module provides ErrorResponse type for HTTP error responses.
//! Note: IntoResponse implementation for AppError lives in the binary crate (mindia-api)
//! due to Rust's orphan rule: external traits (axum::IntoResponse) for external types
//! (mindia_core::AppError) cannot be implemented in library crates.

use serde::Serialize;
use utoipa::ToSchema;

/// Standard error response format for HTTP APIs
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
}
