//! Security audit logging middleware
//!
//! Provides structured audit logging for security-relevant events including:
//! - Authentication attempts (success/failure)
//! - API key creation/revocation
//! - Permission changes
//! - Rate limit violations
//! - File upload/delete operations

#![allow(dead_code)]

use serde::Serialize;
use uuid::Uuid;

/// Audit event types for categorization
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Authentication attempt
    AuthenticationAttempt,
    /// Authentication success
    AuthenticationSuccess,
    /// Authentication failure
    AuthenticationFailure,
    /// API key created
    ApiKeyCreated,
    /// API key revoked
    ApiKeyRevoked,
    /// Permission changed
    PermissionChanged,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// File uploaded
    FileUploaded,
    /// File deleted
    FileDeleted,
    /// Other security events
    SecurityEvent,
}

/// Structured audit log entry
#[derive(Debug, Serialize)]
pub struct AuditLogEntry {
    /// Timestamp of the event
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Type of audit event
    pub event_type: AuditEventType,
    /// Tenant ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<Uuid>,
    /// User ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    /// API key ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_id: Option<Uuid>,
    /// Client IP address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_ip: Option<String>,
    /// User agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    /// HTTP method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_method: Option<String>,
    /// Request path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_path: Option<String>,
    /// Status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
    /// Event details (JSON object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// Success or failure
    pub success: bool,
    /// Error message (if failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

impl AuditLogEntry {
    /// Create a new audit log entry
    pub fn new(event_type: AuditEventType) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            event_type,
            tenant_id: None,
            user_id: None,
            api_key_id: None,
            client_ip: None,
            user_agent: None,
            http_method: None,
            request_path: None,
            status_code: None,
            details: None,
            success: true,
            error_message: None,
        }
    }

    /// Set tenant ID
    pub fn with_tenant_id(mut self, tenant_id: Uuid) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Set API key ID
    pub fn with_api_key_id(mut self, api_key_id: Uuid) -> Self {
        self.api_key_id = Some(api_key_id);
        self
    }

    /// Set client IP
    pub fn with_client_ip(mut self, client_ip: String) -> Self {
        self.client_ip = Some(client_ip);
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = Some(user_agent);
        self
    }

    /// Set HTTP method
    pub fn with_http_method(mut self, method: String) -> Self {
        self.http_method = Some(method);
        self
    }

    /// Set request path
    pub fn with_request_path(mut self, path: String) -> Self {
        self.request_path = Some(path);
        self
    }

    /// Set status code
    pub fn with_status_code(mut self, status_code: u16) -> Self {
        self.status_code = Some(status_code);
        self
    }

    /// Set details
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Mark as failure
    pub fn with_failure(mut self, error_message: String) -> Self {
        self.success = false;
        self.error_message = Some(error_message);
        self
    }

    /// Log the audit entry
    ///
    /// Uses structured logging with the `audit` target for easy filtering
    pub fn log(&self) {
        // Log as JSON for structured logging (useful for log aggregation systems)
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());

        if self.success {
            tracing::event!(
                target: "audit",
                tracing::Level::INFO,
                audit_entry = %json,
                event_type = ?self.event_type,
                tenant_id = ?self.tenant_id,
                user_id = ?self.user_id,
                success = self.success,
                "Security audit log"
            );
        } else {
            tracing::event!(
                target: "audit",
                tracing::Level::WARN,
                audit_entry = %json,
                event_type = ?self.event_type,
                tenant_id = ?self.tenant_id,
                user_id = ?self.user_id,
                success = self.success,
                error = ?self.error_message,
                "Security audit log - failure"
            );
        }
    }
}

// Helper functions for common audit events
/// Log authentication attempt
pub fn log_authentication_attempt(
    tenant_id: Option<Uuid>,
    user_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    client_ip: Option<String>,
    user_agent: Option<String>,
    success: bool,
    error_message: Option<String>,
) {
    let event_type = if success {
        AuditEventType::AuthenticationSuccess
    } else {
        AuditEventType::AuthenticationFailure
    };

    let mut entry = AuditLogEntry::new(event_type)
        .with_tenant_id_opt(tenant_id)
        .with_user_id_opt(user_id)
        .with_api_key_id_opt(api_key_id)
        .with_client_ip_opt(client_ip)
        .with_user_agent_opt(user_agent);

    if !success {
        entry = entry
            .with_failure(error_message.unwrap_or_else(|| "Authentication failed".to_string()));
    }

    entry.log();
}

/// Log API key creation
pub fn log_api_key_created(
    tenant_id: Uuid,
    api_key_id: Uuid,
    user_id: Option<Uuid>,
    client_ip: Option<String>,
) {
    AuditLogEntry::new(AuditEventType::ApiKeyCreated)
        .with_tenant_id(tenant_id)
        .with_api_key_id(api_key_id)
        .with_user_id_opt(user_id)
        .with_client_ip_opt(client_ip)
        .log();
}

/// Log API key revocation
pub fn log_api_key_revoked(
    tenant_id: Uuid,
    api_key_id: Uuid,
    user_id: Option<Uuid>,
    client_ip: Option<String>,
) {
    AuditLogEntry::new(AuditEventType::ApiKeyRevoked)
        .with_tenant_id(tenant_id)
        .with_api_key_id(api_key_id)
        .with_user_id_opt(user_id)
        .with_client_ip_opt(client_ip)
        .log();
}

/// Log rate limit violation
pub fn log_rate_limit_exceeded(
    tenant_id: Option<Uuid>,
    client_ip: Option<String>,
    request_path: Option<String>,
    limit: u32,
) {
    AuditLogEntry::new(AuditEventType::RateLimitExceeded)
        .with_tenant_id_opt(tenant_id)
        .with_client_ip_opt(client_ip)
        .with_request_path_opt(request_path)
        .with_details(serde_json::json!({ "rate_limit": limit }))
        .with_failure("Rate limit exceeded".to_string())
        .log();
}

/// Log file upload
pub fn log_file_upload(
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    file_id: Uuid,
    filename: String,
    file_size: i64,
    content_type: String,
    client_ip: Option<String>,
) {
    AuditLogEntry::new(AuditEventType::FileUploaded)
        .with_tenant_id(tenant_id)
        .with_user_id_opt(user_id)
        .with_client_ip_opt(client_ip)
        .with_details(serde_json::json!({
            "file_id": file_id,
            "filename": filename,
            "file_size": file_size,
            "content_type": content_type,
        }))
        .log();
}

/// Log file deletion
pub fn log_file_deleted(
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    file_id: Uuid,
    filename: String,
    client_ip: Option<String>,
) {
    AuditLogEntry::new(AuditEventType::FileDeleted)
        .with_tenant_id(tenant_id)
        .with_user_id_opt(user_id)
        .with_client_ip_opt(client_ip)
        .with_details(serde_json::json!({
            "file_id": file_id,
            "filename": filename,
        }))
        .log();
}

// Helper trait extensions for optional setters
trait AuditLogEntryOpt {
    fn with_tenant_id_opt(self, tenant_id: Option<Uuid>) -> Self;
    fn with_user_id_opt(self, user_id: Option<Uuid>) -> Self;
    fn with_api_key_id_opt(self, api_key_id: Option<Uuid>) -> Self;
    fn with_client_ip_opt(self, client_ip: Option<String>) -> Self;
    fn with_user_agent_opt(self, user_agent: Option<String>) -> Self;
    fn with_request_path_opt(self, request_path: Option<String>) -> Self;
}

impl AuditLogEntryOpt for AuditLogEntry {
    fn with_tenant_id_opt(mut self, tenant_id: Option<Uuid>) -> Self {
        if let Some(id) = tenant_id {
            self.tenant_id = Some(id);
        }
        self
    }

    fn with_user_id_opt(mut self, user_id: Option<Uuid>) -> Self {
        if let Some(id) = user_id {
            self.user_id = Some(id);
        }
        self
    }

    fn with_api_key_id_opt(mut self, api_key_id: Option<Uuid>) -> Self {
        if let Some(id) = api_key_id {
            self.api_key_id = Some(id);
        }
        self
    }

    fn with_client_ip_opt(mut self, client_ip: Option<String>) -> Self {
        if let Some(ip) = client_ip {
            self.client_ip = Some(ip);
        }
        self
    }

    fn with_user_agent_opt(mut self, user_agent: Option<String>) -> Self {
        if let Some(ua) = user_agent {
            self.user_agent = Some(ua);
        }
        self
    }

    fn with_request_path_opt(mut self, request_path: Option<String>) -> Self {
        if let Some(path) = request_path {
            self.request_path = Some(path);
        }
        self
    }
}
