#![allow(dead_code)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, Row, postgres::PgRow};

use crate::domain::{
    entity::admin_user::{AdminAccountStatus, AdminUser},
    repository::admin_user_repository::{
        AdminUserRepository as AdminUserRepositoryTrait, CreateAdminUser,
    },
};

#[derive(Clone)]
pub struct AdminUserRepository {
    pool: PgPool,
}

const CREATE_ADMIN_USER_SQL: &str = r#"
    insert into admin_users (
        email,
        password_hash,
        display_name,
        account_status,
        last_login_at
    )
    values ($1, $2, $3, $4, $5)
    returning
        id,
        email,
        password_hash,
        display_name,
        account_status,
        last_login_at,
        created_at,
        updated_at
"#;

const FIND_ADMIN_USER_BY_ID_SQL: &str = r#"
    select
        id,
        email,
        password_hash,
        display_name,
        account_status,
        last_login_at,
        created_at,
        updated_at
    from admin_users
    where id = $1
"#;

const FIND_ADMIN_USER_BY_EMAIL_SQL: &str = r#"
    select
        id,
        email,
        password_hash,
        display_name,
        account_status,
        last_login_at,
        created_at,
        updated_at
    from admin_users
    where email = $1
"#;

const TOUCH_ADMIN_USER_LAST_LOGIN_SQL: &str = r#"
    update admin_users
    set
        last_login_at = $2,
        updated_at = now()
    where id = $1
"#;

impl AdminUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_admin_user(row: PgRow) -> Result<AdminUser> {
        let account_status = row
            .try_get::<String, _>("account_status")
            .context("failed to decode admin_users.account_status")?;

        Ok(AdminUser {
            id: row
                .try_get("id")
                .context("failed to decode admin_users.id")?,
            email: row
                .try_get("email")
                .context("failed to decode admin_users.email")?,
            password_hash: row
                .try_get("password_hash")
                .context("failed to decode admin_users.password_hash")?,
            display_name: row
                .try_get("display_name")
                .context("failed to decode admin_users.display_name")?,
            account_status: AdminAccountStatus::from_db(&account_status)?,
            last_login_at: row
                .try_get("last_login_at")
                .context("failed to decode admin_users.last_login_at")?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode admin_users.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode admin_users.updated_at")?,
        })
    }
}

impl AdminUserRepositoryTrait for AdminUserRepository {
    #[tracing::instrument(skip(self, admin_user), fields(admin_user.email = %admin_user.email))]
    async fn create(&self, admin_user: CreateAdminUser) -> Result<AdminUser> {
        let row = sqlx::query(CREATE_ADMIN_USER_SQL)
            .bind(admin_user.email)
            .bind(admin_user.password_hash)
            .bind(admin_user.display_name)
            .bind(admin_user.account_status)
            .bind(admin_user.last_login_at)
            .fetch_one(&self.pool)
            .await
            .context("failed to insert admin user")?;

        Self::map_admin_user(row)
    }

    #[tracing::instrument(skip(self, tx, admin_user), fields(admin_user.email = %admin_user.email))]
    async fn create_tx(
        &self,
        tx: &mut PgConnection,
        admin_user: CreateAdminUser,
    ) -> Result<AdminUser> {
        let row = sqlx::query(CREATE_ADMIN_USER_SQL)
            .bind(admin_user.email)
            .bind(admin_user.password_hash)
            .bind(admin_user.display_name)
            .bind(admin_user.account_status)
            .bind(admin_user.last_login_at)
            .fetch_one(tx)
            .await
            .context("failed to insert admin user")?;

        Self::map_admin_user(row)
    }

    #[tracing::instrument(skip(self), fields(admin_user_id = id))]
    async fn find_by_id(&self, id: i64) -> Result<Option<AdminUser>> {
        let row = sqlx::query(FIND_ADMIN_USER_BY_ID_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query admin user by id")?;

        row.map(Self::map_admin_user).transpose()
    }

    #[tracing::instrument(skip(self, tx), fields(admin_user_id = id))]
    async fn find_by_id_tx(&self, tx: &mut PgConnection, id: i64) -> Result<Option<AdminUser>> {
        let row = sqlx::query(FIND_ADMIN_USER_BY_ID_SQL)
            .bind(id)
            .fetch_optional(tx)
            .await
            .context("failed to query admin user by id")?;

        row.map(Self::map_admin_user).transpose()
    }

    #[tracing::instrument(skip(self, email), fields(admin_user.email = %email))]
    async fn find_by_email(&self, email: &str) -> Result<Option<AdminUser>> {
        let row = sqlx::query(FIND_ADMIN_USER_BY_EMAIL_SQL)
            .bind(email)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query admin user by email")?;

        row.map(Self::map_admin_user).transpose()
    }

    #[tracing::instrument(skip(self, tx, email), fields(admin_user.email = %email))]
    async fn find_by_email_tx(
        &self,
        tx: &mut PgConnection,
        email: &str,
    ) -> Result<Option<AdminUser>> {
        let row = sqlx::query(FIND_ADMIN_USER_BY_EMAIL_SQL)
            .bind(email)
            .fetch_optional(tx)
            .await
            .context("failed to query admin user by email")?;

        row.map(Self::map_admin_user).transpose()
    }

    #[tracing::instrument(skip(self), fields(admin_user_id = id))]
    async fn touch_last_login(&self, id: i64, last_login_at: DateTime<Utc>) -> Result<()> {
        sqlx::query(TOUCH_ADMIN_USER_LAST_LOGIN_SQL)
            .bind(id)
            .bind(last_login_at)
            .execute(&self.pool)
            .await
            .context("failed to touch admin_users.last_login_at")?;

        Ok(())
    }

    #[tracing::instrument(skip(self, tx), fields(admin_user_id = id))]
    async fn touch_last_login_tx(
        &self,
        tx: &mut PgConnection,
        id: i64,
        last_login_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(TOUCH_ADMIN_USER_LAST_LOGIN_SQL)
            .bind(id)
            .bind(last_login_at)
            .execute(tx)
            .await
            .context("failed to touch admin_users.last_login_at")?;

        Ok(())
    }
}
