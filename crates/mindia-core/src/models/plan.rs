use super::subscription::SubscriptionPlan;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Plan limits for a subscription plan
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct PlanLimits {
    pub plan: SubscriptionPlan,
    pub name: String,
    pub description: Option<String>,
    pub max_storage_bytes: i64,
    pub max_api_requests_per_month: i32,
    pub max_uploads_per_month: i32,
    pub max_file_size_bytes: i64,
    pub max_concurrent_transcodes: i32,
    pub max_webhooks: i32,
    pub max_api_keys: i32,
    pub max_organization_members: i32,
    pub features: serde_json::Value,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Plan pricing information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct PlanPricing {
    pub id: Uuid,
    pub plan: SubscriptionPlan,
    pub stripe_price_id: Option<String>,
    pub stripe_yearly_price_id: Option<String>,
    pub currency: String,
    pub monthly_amount_cents: Option<i32>,
    pub yearly_amount_cents: Option<i32>,
    pub trial_days: i32,
    pub setup_fee_cents: i32,
    pub is_active: bool,
    pub effective_from: DateTime<Utc>,
    pub effective_until: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
