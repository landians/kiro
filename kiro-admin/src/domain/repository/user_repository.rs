#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::domain::entity::user::{AccountStatus, User};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListUsers {
    pub uid: Option<i64>,
    pub user_name: Option<String>,
    pub account_status: Option<AccountStatus>,
    pub page: u64,
    pub page_size: u64,
}

impl ListUsers {
    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.page_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginatedUsers {
    pub items: Vec<User>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUserStatus {
    pub account_status: AccountStatus,
    pub frozen_at: Option<DateTime<Utc>>,
    pub banned_at: Option<DateTime<Utc>>,
}

pub trait UserRepository: Send + Sync {
    async fn list(&self, query: &ListUsers) -> Result<PaginatedUsers>;
    async fn find_by_id(&self, id: i64) -> Result<Option<User>>;
    async fn update_status(&self, id: i64, update: UpdateUserStatus) -> Result<User>;
}
