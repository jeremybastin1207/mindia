use chrono::{DateTime, Duration, Utc};
use mindia_core::AppError;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres};
use uuid::Uuid;

/// API Key database model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub key_hash: String,
    pub key_prefix: String,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to create a new API key
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Clone)]
pub struct ApiKeyRepository {
    pool: PgPool,
}

impl ApiKeyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new API key
    #[tracing::instrument(skip(self, key_hash), fields(db.table = "api_keys", db.operation = "insert"))]
    pub async fn create_api_key(
        &self,
        tenant_id: Uuid,
        request: &CreateApiKeyRequest,
        key_hash: String,
        key_prefix: String,
    ) -> Result<ApiKey, AppError> {
        let expires_at = request
            .expires_in_days
            .map(|days| Utc::now() + Duration::days(days));

        let api_key = sqlx::query_as::<Postgres, ApiKey>(
            r#"
            INSERT INTO api_keys (
                tenant_id, name, description, key_hash, key_prefix, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(tenant_id)
        .bind(&request.name)
        .bind(&request.description)
        .bind(&key_hash)
        .bind(&key_prefix)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create API key");
            AppError::Database(e)
        })?;

        tracing::info!(
            api_key_id = %api_key.id,
            tenant_id = %tenant_id,
            name = %request.name,
            "API key created"
        );

        Ok(api_key)
    }

    /// Get API key by hash
    #[tracing::instrument(skip(self, key_hash), fields(db.table = "api_keys", db.operation = "select"))]
    pub async fn get_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, AppError> {
        let api_key = sqlx::query_as::<Postgres, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE key_hash = $1 AND is_active = true
            "#,
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get API key by hash");
            AppError::Database(e)
        })?;

        Ok(api_key)
    }

    /// Get API key by prefix (for lookup)
    #[tracing::instrument(skip(self), fields(db.table = "api_keys", db.operation = "select"))]
    pub async fn get_by_key_prefix(&self, key_prefix: &str) -> Result<Vec<ApiKey>, AppError> {
        let api_keys = sqlx::query_as::<Postgres, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE key_prefix = $1 AND is_active = true
            "#,
        )
        .bind(key_prefix)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get API keys by prefix");
            AppError::Database(e)
        })?;

        Ok(api_keys)
    }

    /// List API keys for a tenant
    #[tracing::instrument(skip(self), fields(db.table = "api_keys", db.operation = "select"))]
    pub async fn list_by_tenant(
        &self,
        tenant_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ApiKey>, AppError> {
        let api_keys = sqlx::query_as::<Postgres, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, tenant_id = %tenant_id, "Failed to list API keys");
            AppError::Database(e)
        })?;

        Ok(api_keys)
    }

    /// Get API key by ID
    #[tracing::instrument(skip(self), fields(db.table = "api_keys", db.operation = "select"))]
    pub async fn get_by_id(&self, tenant_id: Uuid, id: Uuid) -> Result<Option<ApiKey>, AppError> {
        let api_key = sqlx::query_as::<Postgres, ApiKey>(
            r#"
            SELECT * FROM api_keys
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, id = %id, tenant_id = %tenant_id, "Failed to get API key");
            AppError::Database(e)
        })?;

        Ok(api_key)
    }

    /// Revoke (deactivate) an API key
    #[tracing::instrument(skip(self), fields(db.table = "api_keys", db.operation = "update"))]
    pub async fn revoke_api_key(&self, tenant_id: Uuid, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query(
            r#"
            UPDATE api_keys
            SET is_active = false, updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, id = %id, "Failed to revoke API key");
            AppError::Database(e)
        })?;

        let revoked = result.rows_affected() > 0;

        if revoked {
            tracing::info!(
                api_key_id = %id,
                tenant_id = %tenant_id,
                "API key revoked"
            );
        }

        Ok(revoked)
    }

    /// Update last_used_at timestamp
    #[tracing::instrument(skip(self), fields(db.table = "api_keys", db.operation = "update"))]
    pub async fn update_last_used(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE api_keys
            SET last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, id = %id, "Failed to update API key last_used_at");
            AppError::Database(e)
        })?;

        Ok(())
    }

    /// Check if an API key is expired
    pub fn is_expired(api_key: &ApiKey) -> bool {
        if let Some(expires_at) = api_key.expires_at {
            expires_at < Utc::now()
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    #[test]
    fn test_is_expired_with_expired_key() {
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            description: None,
            key_hash: "hash".to_string(),
            key_prefix: "mk_live_".to_string(),
            last_used_at: None,
            expires_at: Some(Utc::now() - Duration::days(1)), // Expired yesterday
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(ApiKeyRepository::is_expired(&api_key));
    }

    #[test]
    fn test_is_expired_with_valid_key() {
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            description: None,
            key_hash: "hash".to_string(),
            key_prefix: "mk_live_".to_string(),
            last_used_at: None,
            expires_at: Some(Utc::now() + Duration::days(1)), // Expires tomorrow
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!ApiKeyRepository::is_expired(&api_key));
    }

    #[test]
    fn test_is_expired_with_no_expiration() {
        let api_key = ApiKey {
            id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            description: None,
            key_hash: "hash".to_string(),
            key_prefix: "mk_live_".to_string(),
            last_used_at: None,
            expires_at: None, // No expiration
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!ApiKeyRepository::is_expired(&api_key));
    }
}
