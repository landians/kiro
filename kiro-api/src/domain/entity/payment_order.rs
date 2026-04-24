#![allow(dead_code)]

use std::fmt::{Display, Formatter};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

use crate::domain::entity::product::{BillingInterval, ChargeType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentOrder {
    pub id: i64,
    pub order_no: String,
    pub user_id: i64,
    pub product_id: i64,
    pub product_plan_id: i64,
    pub payment_provider: PaymentProvider,
    pub order_status: PaymentOrderStatus,
    pub provider_checkout_session_id: Option<String>,
    pub provider_payment_id: Option<String>,
    pub provider_customer_id: Option<String>,
    pub product_code: String,
    pub product_name: String,
    pub product_image_url: Option<String>,
    pub plan_code: String,
    pub plan_name: String,
    pub charge_type: ChargeType,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<BillingInterval>,
    pub trial_days: i32,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentProvider {
    Stripe,
    Creem,
}

impl PaymentProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stripe => "stripe",
            Self::Creem => "creem",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "stripe" => Ok(Self::Stripe),
            "creem" => Ok(Self::Creem),
            other => Err(anyhow!("unsupported payment provider: {other}")),
        }
    }
}

impl Display for PaymentProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaymentOrderStatus {
    Pending,
    Paid,
    Failed,
    Canceled,
    Refunded,
}

impl PaymentOrderStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Paid => "paid",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
            Self::Refunded => "refunded",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "paid" => Ok(Self::Paid),
            "failed" => Ok(Self::Failed),
            "canceled" => Ok(Self::Canceled),
            "refunded" => Ok(Self::Refunded),
            other => Err(anyhow!("unsupported payment order status: {other}")),
        }
    }
}

impl Display for PaymentOrderStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
