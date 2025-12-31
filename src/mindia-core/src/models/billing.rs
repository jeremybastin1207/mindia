use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Payment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "payment_status", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Succeeded,
    Pending,
    Failed,
    Refunded,
}

/// Payment type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "payment_type", rename_all = "lowercase")
)]
#[serde(rename_all = "lowercase")]
pub enum PaymentType {
    Subscription,
    OneTime,
    Refund,
}

/// Billing history entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct BillingHistory {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub subscription_id: Option<Uuid>,
    pub amount_cents: i32,
    pub currency: String,
    pub status: PaymentStatus,
    pub payment_type: PaymentType,
    pub stripe_payment_intent_id: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub description: Option<String>,
    pub invoice_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
