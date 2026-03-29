#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::domain::entity::user::User;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateUser {
    pub primary_email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUserProfile {
    pub primary_email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub last_login_at: DateTime<Utc>,
}

pub trait UserRepository: Send + Sync {
    async fn create(&self, user: CreateUser) -> Result<User>;

    async fn create_tx(&self, tx: &mut PgConnection, user: CreateUser) -> Result<User>;

    async fn find_by_id(&self, id: i64) -> Result<Option<User>>;

    async fn find_by_id_tx(&self, tx: &mut PgConnection, id: i64) -> Result<Option<User>>;

    async fn update_profile(&self, id: i64, profile: UpdateUserProfile) -> Result<User>;

    async fn update_profile_tx(
        &self,
        tx: &mut PgConnection,
        id: i64,
        profile: UpdateUserProfile,
    ) -> Result<User>;

    async fn touch_last_login(&self, id: i64, last_login_at: DateTime<Utc>) -> Result<()>;

    async fn touch_last_login_tx(
        &self,
        tx: &mut PgConnection,
        id: i64,
        last_login_at: DateTime<Utc>,
    ) -> Result<()>;
}
