//! API constants and configuration
//!
//! This module defines constants used throughout the API, including versioning.
//! API version is set at build time via the `API_VERSION` environment variable (default: v0).
//! Generated constants are in `api_version.rs` (built by build.rs).
//!
//! # Changing the API Version
//!
//! Set `API_VERSION` when building, e.g. `API_VERSION=v1 cargo build`.
//! Routes and OpenAPI spec use the version automatically; handler path annotations
//! use a placeholder that is transformed at runtime in the OpenAPI spec.

#![allow(dead_code)]

/// API base path prefix (version-independent)
pub const API_BASE: &str = "/api";

// Version and versioned prefixes: generated at build time by build.rs
include!(concat!(env!("OUT_DIR"), "/api_version.rs"));

/// Helper macro for constructing API paths with version prefix at compile time.
/// Uses API_VERSION_STR set by build.rs (from API_VERSION env, default v0).
///
/// Usage: `api_path!("/images")` expands to `"/api/v0/images"` when API_VERSION=v0.
#[macro_export]
macro_rules! api_path {
    ($path:expr) => {
        concat!("/api/", env!("API_VERSION_STR"), $path)
    };
}
