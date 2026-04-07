#![allow(dead_code)]

use anyhow::Result;

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

pub trait UserRepository: Send + Sync {
    async fn list(&self, query: &ListUsers) -> Result<PaginatedUsers>;
}
