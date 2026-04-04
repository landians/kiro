#![allow(dead_code)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use crate::domain::entity::admin_user::AdminUser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAdminUser {
    pub email: String,
    pub password_hash: String,
    pub display_name: Option<String>,
    pub account_status: String,
    pub last_login_at: Option<DateTime<Utc>>,
}

pub trait AdminUserRepository: Send + Sync {
    async fn create(&self, admin_user: CreateAdminUser) -> Result<AdminUser>;

    async fn create_tx(
        &self,
        tx: &mut PgConnection,
        admin_user: CreateAdminUser,
    ) -> Result<AdminUser>;

    async fn find_by_id(&self, id: i64) -> Result<Option<AdminUser>>;

    async fn find_by_id_tx(&self, tx: &mut PgConnection, id: i64) -> Result<Option<AdminUser>>;

    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>>;

    async fn find_by_email_tx(
        &self,
        tx: &mut PgConnection,
        email: &str,
    ) -> Result<Option<AdminUser>>;

    async fn touch_last_login(&self, id: i64, last_login_at: DateTime<Utc>) -> Result<()>;

    async fn touch_last_login_tx(
        &self,
        tx: &mut PgConnection,
        id: i64,
        last_login_at: DateTime<Utc>,
    ) -> Result<()>;
}
