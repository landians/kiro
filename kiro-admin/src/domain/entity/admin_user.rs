#![allow(dead_code)]

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminUser {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub account_status: AdminAccountStatus,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AdminUser {
    pub fn is_active(&self) -> bool {
        self.account_status == AdminAccountStatus::Active
    }

    pub fn is_frozen(&self) -> bool {
        self.account_status == AdminAccountStatus::Frozen
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminAccountStatus {
    Active,
    Frozen,
}

impl AdminAccountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Frozen => "frozen",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "frozen" => Ok(Self::Frozen),
            other => Err(anyhow!("unsupported admin account status: {other}")),
        }
    }
}
