use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// User entity from OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub auth_provider: String,
    pub auth_provider_id: String,
    pub auth_provider_email: Option<String>,
    pub name: Option<String>,
    pub picture_url: Option<String>,
    pub email_verified: bool,
    pub is_active: bool,
    pub is_system_admin: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// User session entity
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct UserSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: String,
    pub provider_access_token_encrypted: Option<String>,
    pub provider_refresh_token_encrypted: Option<String>,
    pub provider_id_token_hash: Option<String>,
    pub provider_token_expires_at: Option<DateTime<Utc>>,
    pub mindia_jwt_token_id: Option<String>,
    pub last_used_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

/// Auth provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct AuthProvider {
    pub id: Uuid,
    pub provider_name: String,
    pub display_name: String,
    pub is_enabled: bool,
    pub client_id: String,
    pub client_secret_encrypted: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub jwks_url: Option<String>,
    pub scopes: String,
    pub config: serde_json::Value,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
