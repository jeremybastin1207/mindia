//! Hooks and traits for SaaS integration
//!
//! This module provides trait interfaces that allow the OSS core to work with
//! SaaS management features (billing, usage tracking, etc.) without directly
//! depending on them. The SaaS layer implements these traits.

use async_trait::async_trait;
use uuid::Uuid;

/// Tenant context information
///
/// This is a minimal interface for tenant information that the OSS core needs.
/// The SaaS layer provides a full implementation.
#[derive(Debug, Clone)]
pub struct TenantContextInfo {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

/// Usage information for a tenant
#[derive(Debug, Clone)]
pub struct UsageInfo {
    pub storage_bytes_used: i64,
    pub storage_bytes_limit: i64,
    pub api_requests_count: i32,
    pub api_requests_limit: i32,
}

/// Trait for reporting usage to the SaaS layer
///
/// The OSS core calls these methods to report usage events. The SaaS layer
/// implements this trait to track usage for billing and limits.
#[async_trait]
pub trait UsageReporter: Send + Sync {
    /// Report storage usage change
    async fn report_storage_change(&self, tenant_id: Uuid, bytes_delta: i64) -> Result<(), String>;

    /// Report API request
    async fn report_api_request(&self, tenant_id: Uuid) -> Result<(), String>;

    /// Get current usage information
    async fn get_usage(&self, tenant_id: Uuid) -> Result<Option<UsageInfo>, String>;

    /// Check if storage limit would be exceeded
    async fn check_storage_limit(
        &self,
        tenant_id: Uuid,
        additional_bytes: i64,
    ) -> Result<Option<String>, String>;

    /// Check if API request limit would be exceeded
    async fn check_api_limit(&self, tenant_id: Uuid) -> Result<Option<String>, String>;
}

/// No-op implementation for when SaaS features are disabled
pub struct NoOpUsageReporter;

#[async_trait]
impl UsageReporter for NoOpUsageReporter {
    async fn report_storage_change(
        &self,
        _tenant_id: Uuid,
        _bytes_delta: i64,
    ) -> Result<(), String> {
        Ok(())
    }

    async fn report_api_request(&self, _tenant_id: Uuid) -> Result<(), String> {
        Ok(())
    }

    async fn get_usage(&self, _tenant_id: Uuid) -> Result<Option<UsageInfo>, String> {
        Ok(None)
    }

    async fn check_storage_limit(
        &self,
        _tenant_id: Uuid,
        _additional_bytes: i64,
    ) -> Result<Option<String>, String> {
        Ok(None)
    }

    async fn check_api_limit(&self, _tenant_id: Uuid) -> Result<Option<String>, String> {
        Ok(None)
    }
}
