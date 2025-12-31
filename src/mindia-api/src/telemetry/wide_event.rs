//! Wide event logging with tail sampling
//!
//! Implements the "wide events" / "canonical log lines" pattern described in
//! https://loggingsucks.com/. Instead of logging what the code is doing,
//! we log what happened to each request with full context.

#![allow(dead_code)]

use crate::auth::models::UserRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Wide event structure capturing all context for a single request
#[derive(Debug, Clone, Serialize)]
pub struct WideEvent {
    // Request identification
    #[serde(rename = "request_id")]
    pub request_id: String,
    #[serde(rename = "trace_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(rename = "timestamp")]
    pub timestamp: DateTime<Utc>,

    // Service context
    #[serde(rename = "service")]
    pub service: String,
    #[serde(rename = "version")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(rename = "deployment_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    #[serde(rename = "region")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(rename = "environment")]
    pub environment: String,

    // HTTP request context
    #[serde(rename = "method")]
    pub method: String,
    #[serde(rename = "path")]
    pub path: String,
    #[serde(rename = "normalized_path")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_path: Option<String>,
    #[serde(rename = "query_string")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_string: Option<String>,
    #[serde(rename = "status_code")]
    pub status_code: u16,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    #[serde(rename = "outcome")]
    pub outcome: Outcome,

    // Client context
    #[serde(rename = "client_ip")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,
    #[serde(rename = "user_agent")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,

    // Request/Response sizes
    #[serde(rename = "request_size_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_size_bytes: Option<u64>,
    #[serde(rename = "response_size_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_size_bytes: Option<u64>,

    // Tenant and user context (high cardinality fields)
    #[serde(rename = "tenant")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<TenantInfo>,
    #[serde(rename = "user")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserContext>,

    // Business context (enriched during request processing)
    #[serde(rename = "business")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business: Option<BusinessContext>,

    // Error context
    #[serde(rename = "error")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorContext>,

    // Performance context
    #[serde(rename = "performance")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceContext>,

    // Database context
    #[serde(rename = "database")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseContext>,

    // Storage/S3 context
    #[serde(rename = "storage")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageContext>,

    // External API calls
    #[serde(rename = "external_calls")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_calls: Option<Vec<ExternalCall>>,

    // Feature flags or configuration
    #[serde(rename = "feature_flags")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_flags: Option<HashMap<String, bool>>,

    // Custom fields for extensibility
    #[serde(rename = "custom")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<HashMap<String, serde_json::Value>>,
}

/// Outcome of the request
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Success,
    Error,
    ClientError,
}

impl Outcome {
    pub fn from_status_code(status: u16) -> Self {
        if status >= 500 {
            Outcome::Error
        } else if status >= 400 {
            Outcome::ClientError
        } else {
            Outcome::Success
        }
    }
}

/// Tenant context in wide event
#[derive(Debug, Clone, Serialize)]
pub struct TenantInfo {
    #[serde(rename = "id")]
    pub id: Uuid,
    #[serde(rename = "name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "status")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(rename = "created_at")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
}

impl From<&crate::auth::models::TenantContext> for TenantInfo {
    fn from(ctx: &crate::auth::models::TenantContext) -> Self {
        Self {
            id: ctx.tenant_id,
            name: Some(ctx.tenant.name.clone()),
            status: Some(format!("{:?}", ctx.tenant.status)),
            created_at: Some(ctx.tenant.created_at),
        }
    }
}

/// User context in wide event
#[derive(Debug, Clone, Serialize)]
pub struct UserContext {
    #[serde(rename = "id")]
    pub id: Uuid,
    #[serde(rename = "role")]
    pub role: String,
    #[serde(rename = "account_age_days")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_age_days: Option<i64>,
}

impl From<&crate::auth::models::TenantContext> for UserContext {
    fn from(ctx: &crate::auth::models::TenantContext) -> Self {
        let role_str = match ctx.role {
            UserRole::Admin => "admin",
            UserRole::Member => "member",
            UserRole::Viewer => "viewer",
        };

        Self {
            id: ctx.user_id,
            role: role_str.to_string(),
            account_age_days: None, // Could be calculated from tenant.created_at if needed
        }
    }
}

/// Business context (enriched by handlers)
#[derive(Debug, Clone, Serialize, Default)]
pub struct BusinessContext {
    #[serde(rename = "media_type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>, // "image", "video", "audio", "document"
    #[serde(rename = "media_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_id: Option<Uuid>,
    #[serde(rename = "file_size")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    #[serde(rename = "operation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>, // "upload", "download", "delete", "transform", etc.
    #[serde(rename = "folder_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder_id: Option<Uuid>,
    #[serde(rename = "task_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<Uuid>,
    #[serde(rename = "plugin_name")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin_name: Option<String>,
    #[serde(rename = "webhook_id")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_id: Option<Uuid>,
}

/// Error context
#[derive(Debug, Clone, Serialize)]
pub struct ErrorContext {
    #[serde(rename = "type")]
    pub error_type: String,
    #[serde(rename = "code")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(rename = "message")]
    pub message: String,
    #[serde(rename = "retriable")]
    pub retriable: bool,
    #[serde(rename = "stack_trace")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_trace: Option<String>,
}

/// Performance context
#[derive(Debug, Clone, Serialize, Default)]
pub struct PerformanceContext {
    #[serde(rename = "db_queries")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_queries: Option<u32>,
    #[serde(rename = "db_total_time_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_total_time_ms: Option<u64>,
    #[serde(rename = "cache_hit")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<bool>,
    #[serde(rename = "slow_query_detected")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slow_query_detected: Option<bool>,
}

/// Database context
#[derive(Debug, Clone, Serialize, Default)]
pub struct DatabaseContext {
    #[serde(rename = "queries_executed")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries_executed: Option<u32>,
    #[serde(rename = "slowest_query_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slowest_query_ms: Option<u64>,
    #[serde(rename = "pool_active")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_active: Option<u32>,
    #[serde(rename = "pool_waiting")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_waiting: Option<u32>,
}

/// Storage context (S3/local)
#[derive(Debug, Clone, Serialize, Default)]
pub struct StorageContext {
    #[serde(rename = "operation")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>, // "upload", "download", "delete", "exists"
    #[serde(rename = "bucket")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(rename = "key")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(rename = "size_bytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(rename = "duration_ms")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(rename = "region")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

/// External API call context
#[derive(Debug, Clone, Serialize)]
pub struct ExternalCall {
    #[serde(rename = "service")]
    pub service: String,
    #[serde(rename = "endpoint")]
    pub endpoint: String,
    #[serde(rename = "method")]
    pub method: String,
    #[serde(rename = "status_code")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    #[serde(rename = "duration_ms")]
    pub duration_ms: u64,
    #[serde(rename = "success")]
    pub success: bool,
    #[serde(rename = "retry_count")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<u32>,
}

impl WideEvent {
    /// Create a new wide event with initial request context
    pub fn new(
        request_id: String,
        service: String,
        environment: String,
        method: String,
        path: String,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            request_id,
            trace_id: None,
            timestamp,
            service,
            version: None,
            deployment_id: None,
            region: None,
            environment,
            method,
            path,
            normalized_path: None,
            query_string: None,
            status_code: 0,            // Will be set when response is available
            duration_ms: 0,            // Will be set when response completes
            outcome: Outcome::Success, // Will be set based on status code
            client_ip: None,
            user_agent: None,
            request_size_bytes: None,
            response_size_bytes: None,
            tenant: None,
            user: None,
            business: None,
            error: None,
            performance: None,
            database: None,
            storage: None,
            external_calls: None,
            feature_flags: None,
            custom: None,
        }
    }

    /// Add tenant and user context
    pub fn with_tenant_context(&mut self, ctx: &crate::auth::models::TenantContext) {
        self.tenant = Some(TenantInfo::from(ctx));
        self.user = Some(UserContext::from(ctx));
    }

    /// Add business context
    pub fn with_business_context<F>(&mut self, f: F)
    where
        F: FnOnce(&mut BusinessContext),
    {
        let mut business = self.business.take().unwrap_or_default();
        f(&mut business);
        self.business = Some(business);
    }

    /// Add error context
    pub fn with_error(&mut self, error: ErrorContext) {
        self.error = Some(error);
        self.outcome = Outcome::Error;
    }

    /// Add performance context
    pub fn with_performance_context<F>(&mut self, f: F)
    where
        F: FnOnce(&mut PerformanceContext),
    {
        let mut perf = self.performance.take().unwrap_or_default();
        f(&mut perf);
        self.performance = Some(perf);
    }

    /// Add database context
    pub fn with_database_context<F>(&mut self, f: F)
    where
        F: FnOnce(&mut DatabaseContext),
    {
        let mut db = self.database.take().unwrap_or_default();
        f(&mut db);
        self.database = Some(db);
    }

    /// Add storage context
    pub fn with_storage_context<F>(&mut self, f: F)
    where
        F: FnOnce(&mut StorageContext),
    {
        let mut storage = self.storage.take().unwrap_or_default();
        f(&mut storage);
        self.storage = Some(storage);
    }

    /// Add external call
    pub fn add_external_call(&mut self, call: ExternalCall) {
        if let Some(ref mut calls) = self.external_calls {
            calls.push(call);
        } else {
            self.external_calls = Some(vec![call]);
        }
    }

    /// Add custom field
    pub fn add_custom(&mut self, key: String, value: serde_json::Value) {
        if let Some(ref mut custom) = self.custom {
            custom.insert(key, value);
        } else {
            let mut custom = HashMap::new();
            custom.insert(key, value);
            self.custom = Some(custom);
        }
    }

    /// Finalize the event with response information
    pub fn finalize(&mut self, status_code: u16, duration_ms: u64) {
        self.status_code = status_code;
        self.duration_ms = duration_ms;
        self.outcome = Outcome::from_status_code(status_code);
    }

    /// Convert to JSON for logging
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

/// Tail sampling configuration
#[derive(Debug, Clone)]
pub struct TailSamplingConfig {
    /// Always keep errors (100% of 5xx and exceptions)
    pub keep_all_errors: bool,
    /// Always keep client errors (4xx)
    pub keep_all_client_errors: bool,
    /// Always keep requests slower than this threshold (ms)
    pub slow_request_threshold_ms: Option<u64>,
    /// Always keep requests for VIP tenants (if provided)
    pub vip_tenant_ids: Option<Vec<Uuid>>,
    /// Always keep requests with specific paths (for debugging)
    pub keep_paths: Option<Vec<String>>,
    /// Random sampling rate for other requests (0.0 to 1.0)
    pub random_sample_rate: f64,
    /// Always keep requests when enabled
    pub always_keep_enabled: bool,
}

impl Default for TailSamplingConfig {
    fn default() -> Self {
        Self {
            keep_all_errors: true,
            keep_all_client_errors: false,
            slow_request_threshold_ms: Some(2000), // Keep requests slower than 2s (p99 threshold)
            vip_tenant_ids: None,
            keep_paths: None,
            random_sample_rate: 0.05, // Sample 5% of successful requests
            always_keep_enabled: false,
        }
    }
}

impl TailSamplingConfig {
    /// Determine if this event should be kept based on tail sampling rules
    pub fn should_sample(&self, event: &WideEvent) -> bool {
        // Always keep errors
        if self.keep_all_errors && event.outcome == Outcome::Error {
            return true;
        }

        // Always keep client errors if configured
        if self.keep_all_client_errors && event.outcome == Outcome::ClientError {
            return true;
        }

        // Always keep slow requests (above p99 threshold)
        if let Some(threshold) = self.slow_request_threshold_ms {
            if event.duration_ms > threshold {
                return true;
            }
        }

        // Always keep VIP tenants
        if let Some(ref vip_ids) = self.vip_tenant_ids {
            if let Some(ref tenant) = event.tenant {
                if vip_ids.contains(&tenant.id) {
                    return true;
                }
            }
        }

        // Always keep specific paths (for debugging rollouts)
        if let Some(ref keep_paths) = self.keep_paths {
            if keep_paths.iter().any(|p| event.path.starts_with(p)) {
                return true;
            }
        }

        // Always keep if explicitly enabled (for debugging)
        if self.always_keep_enabled {
            return true;
        }

        // Random sample the rest
        // Use a simple hash-based approach for deterministic sampling based on request_id
        // This is better than thread_rng for distributed systems
        use std::hash::{DefaultHasher, Hash, Hasher};

        // Use request_id as seed for consistent sampling per request
        let mut hasher = DefaultHasher::new();
        if let Some(ref tenant) = event.tenant {
            tenant.id.hash(&mut hasher);
        }
        event.request_id.hash(&mut hasher);
        event.path.hash(&mut hasher);

        let hash = hasher.finish();
        // Convert hash to [0.0, 1.0) range
        let random_value = (hash % 10000) as f64 / 10000.0;

        random_value < self.random_sample_rate
    }
}
