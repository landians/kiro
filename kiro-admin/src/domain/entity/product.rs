#![allow(dead_code)]

use std::fmt::{Display, Formatter};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub id: i64,
    pub product_code: String,
    pub product_name: String,
    pub product_description: Option<String>,
    pub product_status: CatalogStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductPlan {
    pub id: i64,
    pub product_id: i64,
    pub plan_code: String,
    pub plan_name: String,
    pub plan_status: CatalogStatus,
    pub charge_type: ChargeType,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<BillingInterval>,
    pub billing_interval_count: Option<i32>,
    pub trial_days: i32,
    pub sort_order: i32,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogStatus {
    Draft,
    Active,
    Inactive,
    Archived,
}

impl CatalogStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Archived => "archived",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "inactive" => Ok(Self::Inactive),
            "archived" => Ok(Self::Archived),
            other => Err(anyhow!("unsupported catalog status: {other}")),
        }
    }
}

impl Display for CatalogStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChargeType {
    OneTime,
    Subscription,
}

impl ChargeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::OneTime => "one_time",
            Self::Subscription => "subscription",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "one_time" => Ok(Self::OneTime),
            "subscription" => Ok(Self::Subscription),
            other => Err(anyhow!("unsupported charge type: {other}")),
        }
    }
}

impl Display for ChargeType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingInterval {
    Day,
    Week,
    Month,
    Year,
}

impl BillingInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Week => "week",
            Self::Month => "month",
            Self::Year => "year",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "day" => Ok(Self::Day),
            "week" => Ok(Self::Week),
            "month" => Ok(Self::Month),
            "year" => Ok(Self::Year),
            other => Err(anyhow!("unsupported billing interval: {other}")),
        }
    }
}

impl Display for BillingInterval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
