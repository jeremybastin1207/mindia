use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Subscription plan type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "subscription_plan", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionPlan {
    Free,
    Starter,
    Professional,
    Enterprise,
}

/// Subscription status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "subscription_status", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    PastDue,
    Trialing,
    Incomplete,
}

/// Subscription entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Subscription {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub plan: SubscriptionPlan,
    pub status: SubscriptionStatus,
    pub stripe_customer_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub stripe_price_id: Option<String>,
    pub current_period_start: Option<DateTime<Utc>>,
    pub current_period_end: Option<DateTime<Utc>>,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<DateTime<Utc>>,
    pub trial_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
