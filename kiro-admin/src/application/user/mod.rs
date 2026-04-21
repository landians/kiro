pub mod get_admin_user;
pub mod list_users;
pub mod update_user_status;

use anyhow::Result;
use chrono::{DateTime, Utc};
use thiserror::Error;

use self::{
    get_admin_user::GetAdminUserLogic, list_users::ListUsersLogic,
    update_user_status::UpdateUserStatusLogic,
};
use crate::domain::{
    entity::{
        admin_user::AdminUser,
        user::{AccountStatus, User},
    },
    repository::{
        admin_user_repository::AdminUserRepository,
        user_repository::{ListUsers, PaginatedUsers, UserRepository},
    },
};

pub struct AdminUserLogic<AR> {
    get_admin_user_logic: GetAdminUserLogic<AR>,
}

impl<AR> AdminUserLogic<AR>
where
    AR: AdminUserRepository + Clone,
{
    pub fn new(admin_user_repository: AR) -> Self {
        Self {
            get_admin_user_logic: GetAdminUserLogic::new(admin_user_repository),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get(&self, admin_user_id: i64) -> Result<AdminUser> {
        self.get_admin_user_logic.execute(admin_user_id).await
    }
}

pub struct UserLogic<UR> {
    list_users_logic: ListUsersLogic<UR>,
    update_user_status_logic: UpdateUserStatusLogic<UR>,
}

impl<UR> UserLogic<UR>
where
    UR: UserRepository + Clone,
{
    pub fn new(user_repository: UR) -> Self {
        Self {
            list_users_logic: ListUsersLogic::new(user_repository.clone()),
            update_user_status_logic: UpdateUserStatusLogic::new(user_repository),
        }
    }

    #[tracing::instrument(skip(self, query))]
    pub async fn list(&self, query: ListUsers) -> Result<PaginatedUsers> {
        self.list_users_logic.execute(query).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn freeze(&self, user_id: i64, frozen_at: DateTime<Utc>) -> Result<User> {
        self.update_user_status_logic
            .freeze(user_id, frozen_at)
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn ban(&self, user_id: i64, banned_at: DateTime<Utc>) -> Result<User> {
        self.update_user_status_logic.ban(user_id, banned_at).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn activate(&self, user_id: i64) -> Result<User> {
        self.update_user_status_logic.activate(user_id).await
    }
}

#[derive(Debug, Error)]
pub enum AdminUserLogicError {
    #[error("admin user {admin_user_id} not found")]
    AdminUserNotFound { admin_user_id: i64 },
}

#[derive(Debug, Error)]
pub enum UserLogicError {
    #[error("user {user_id} not found")]
    UserNotFound { user_id: i64 },
    #[error("user {user_id} cannot transition from {current_status} to {target_status}")]
    InvalidUserStatusTransition {
        user_id: i64,
        current_status: AccountStatus,
        target_status: AccountStatus,
    },
}
