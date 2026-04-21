use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    domain::entity::user::{AccountStatus, User},
    domain::repository::user_repository::{ListUsers, PaginatedUsers},
};

#[derive(Debug, Deserialize, Validate)]
pub struct ListUsersRequest {
    #[validate(range(min = 1))]
    pub uid: Option<i64>,
    pub user_name: Option<String>,
    pub user_status: Option<UserStatusQuery>,
    #[validate(range(min = 1, max = 10_000))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100))]
    pub page_size: Option<u64>,
}

impl ListUsersRequest {
    const DEFAULT_PAGE: u64 = 1;
    const DEFAULT_PAGE_SIZE: u64 = 20;

    pub fn into_query(self) -> ListUsers {
        ListUsers {
            uid: self.uid,
            user_name: normalize_query_text(self.user_name),
            account_status: self.user_status.map(Into::into),
            page: self.page.unwrap_or(Self::DEFAULT_PAGE),
            page_size: self.page_size.unwrap_or(Self::DEFAULT_PAGE_SIZE),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserStatusQuery {
    Active,
    Frozen,
    Banned,
}

impl From<UserStatusQuery> for AccountStatus {
    fn from(value: UserStatusQuery) -> Self {
        match value {
            UserStatusQuery::Active => Self::Active,
            UserStatusQuery::Frozen => Self::Frozen,
            UserStatusQuery::Banned => Self::Banned,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserListResponse {
    pub items: Vec<UserDto>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

impl From<PaginatedUsers> for UserListResponse {
    fn from(value: PaginatedUsers) -> Self {
        Self {
            items: value.items.into_iter().map(UserDto::from).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub uid: i64,
    pub primary_email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub account_status: &'static str,
    pub frozen_at: Option<String>,
    pub banned_at: Option<String>,
    pub last_login_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<User> for UserDto {
    fn from(value: User) -> Self {
        Self {
            uid: value.id,
            primary_email: value.primary_email,
            email_verified: value.email_verified,
            display_name: value.display_name,
            avatar_url: value.avatar_url,
            account_status: value.account_status.as_str(),
            frozen_at: value.frozen_at.map(|time| time.to_rfc3339()),
            banned_at: value.banned_at.map(|time| time.to_rfc3339()),
            last_login_at: value.last_login_at.map(|time| time.to_rfc3339()),
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserStatusRequest {
    pub account_status: ManageableUserStatus,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManageableUserStatus {
    Active,
    Frozen,
    Banned,
}

fn normalize_query_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        if value.is_empty() {
            return None;
        }

        Some(value)
    })
}
