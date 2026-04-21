#![allow(dead_code)]

use std::fmt::{Display, Formatter};

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub primary_email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub account_status: AccountStatus,
    pub frozen_at: Option<DateTime<Utc>>,
    pub banned_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn is_active(&self) -> bool {
        self.account_status == AccountStatus::Active
    }

    pub fn is_frozen(&self) -> bool {
        self.account_status == AccountStatus::Frozen
    }

    pub fn is_banned(&self) -> bool {
        self.account_status == AccountStatus::Banned
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountStatus {
    Active,
    Frozen,
    Banned,
}

impl AccountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Frozen => "frozen",
            Self::Banned => "banned",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "frozen" => Ok(Self::Frozen),
            "banned" => Ok(Self::Banned),
            other => Err(anyhow!("unsupported account status: {other}")),
        }
    }
}

impl Display for AccountStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
