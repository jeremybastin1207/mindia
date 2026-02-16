use crate::error::ErrorResponse;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, StatusCode};
use axum::Json;
use chrono::{DateTime, Utc};
use mindia_core::models::{Tenant, TenantStatus};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use utoipa::ToSchema;
use uuid::Uuid;

/// User role for authorization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Member,
    Viewer,
}

impl Display for UserRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            UserRole::Admin => write!(f, "admin"),
            UserRole::Member => write!(f, "member"),
            UserRole::Viewer => write!(f, "viewer"),
        }
    }
}

/// JWT claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid, // user_id
    pub tenant_id: Uuid,
    pub role: String, // "admin", "member", or "viewer"
    pub exp: i64,     // expiration timestamp
    pub iat: i64,     // issued at timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nbf: Option<i64>, // not-before timestamp (optional)
}

/// Tenant context extracted from JWT or API key and stored in request extensions
#[derive(Debug, Clone)]
pub struct TenantContext {
    pub tenant_id: Uuid,
    pub user_id: Uuid,  // From JWT claims or API key
    pub role: UserRole, // From JWT claims or API key
    pub tenant: Tenant,
}

/// Tenant information in responses
#[allow(dead_code)]
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResponse {
    pub id: Uuid,
    pub name: String,
    pub status: TenantStatus,
    pub created_at: DateTime<Utc>,
}

// Implement FromRequestParts for TenantContext to work with Multipart
// Extension cannot be used with Multipart, so we extract directly from request parts
impl<S> FromRequestParts<S> for TenantContext
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<TenantContext>()
            .cloned()
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse {
                        error: "Missing tenant context".to_string(),
                        details: None,
                        error_type: None,
                        code: "MISSING_TENANT_CONTEXT".to_string(),
                        recoverable: false,
                        suggested_action: Some("Check authentication token or API key".to_string()),
                    }),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_status_equality() {
        assert_eq!(TenantStatus::Active, TenantStatus::Active);
        assert_ne!(TenantStatus::Active, TenantStatus::Suspended);
        assert_ne!(TenantStatus::Suspended, TenantStatus::Deleted);
    }
}
