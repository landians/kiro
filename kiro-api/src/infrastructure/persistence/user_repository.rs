#![allow(dead_code)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool, Row, postgres::PgRow};

use crate::domain::{
    entity::user::{AccountStatus, User},
    repository::user_repository::{
        CreateUser, UpdateUserProfile, UserRepository as UserRepositoryTrait,
    },
};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

const CREATE_USER_SQL: &str = r#"
    insert into users (
        primary_email,
        email_verified,
        display_name,
        avatar_url,
        last_login_at
    )
    values ($1, $2, $3, $4, $5)
    returning
        id,
        primary_email,
        email_verified,
        display_name,
        avatar_url,
        account_status,
        frozen_at,
        banned_at,
        last_login_at,
        created_at,
        updated_at
"#;

const FIND_USER_BY_ID_SQL: &str = r#"
    select
        id,
        primary_email,
        email_verified,
        display_name,
        avatar_url,
        account_status,
        frozen_at,
        banned_at,
        last_login_at,
        created_at,
        updated_at
    from users
    where id = $1
"#;

const UPDATE_USER_PROFILE_SQL: &str = r#"
    update users
    set
        primary_email = $2,
        email_verified = $3,
        display_name = $4,
        avatar_url = $5,
        last_login_at = $6,
        updated_at = now()
    where id = $1
    returning
        id,
        primary_email,
        email_verified,
        display_name,
        avatar_url,
        account_status,
        frozen_at,
        banned_at,
        last_login_at,
        created_at,
        updated_at
"#;

const TOUCH_USER_LAST_LOGIN_SQL: &str = r#"
    update users
    set
        last_login_at = $2,
        updated_at = now()
    where id = $1
"#;

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_user(row: PgRow) -> Result<User> {
        let account_status = row
            .try_get::<String, _>("account_status")
            .context("failed to decode users.account_status")?;

        Ok(User {
            id: row.try_get("id").context("failed to decode users.id")?,
            primary_email: row
                .try_get("primary_email")
                .context("failed to decode users.primary_email")?,
            email_verified: row
                .try_get("email_verified")
                .context("failed to decode users.email_verified")?,
            display_name: row
                .try_get("display_name")
                .context("failed to decode users.display_name")?,
            avatar_url: row
                .try_get("avatar_url")
                .context("failed to decode users.avatar_url")?,
            account_status: AccountStatus::from_db(&account_status)?,
            frozen_at: row
                .try_get("frozen_at")
                .context("failed to decode users.frozen_at")?,
            banned_at: row
                .try_get("banned_at")
                .context("failed to decode users.banned_at")?,
            last_login_at: row
                .try_get("last_login_at")
                .context("failed to decode users.last_login_at")?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode users.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode users.updated_at")?,
        })
    }
}

impl UserRepositoryTrait for UserRepository {
    #[tracing::instrument(skip(self, user))]
    async fn create(&self, user: CreateUser) -> Result<User> {
        let row = sqlx::query(CREATE_USER_SQL)
            .bind(user.primary_email)
            .bind(user.email_verified)
            .bind(user.display_name)
            .bind(user.avatar_url)
            .bind(user.last_login_at)
            .fetch_one(&self.pool)
            .await
            .context("failed to insert user")?;

        Self::map_user(row)
    }

    #[tracing::instrument(skip(self, tx, user))]
    async fn create_tx(&self, tx: &mut PgConnection, user: CreateUser) -> Result<User> {
        let row = sqlx::query(CREATE_USER_SQL)
            .bind(user.primary_email)
            .bind(user.email_verified)
            .bind(user.display_name)
            .bind(user.avatar_url)
            .bind(user.last_login_at)
            .fetch_one(tx)
            .await
            .context("failed to insert user")?;

        Self::map_user(row)
    }

    #[tracing::instrument(skip(self), fields(user_id = id))]
    async fn find_by_id(&self, id: i64) -> Result<Option<User>> {
        let row = sqlx::query(FIND_USER_BY_ID_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query user by id")?;

        row.map(Self::map_user).transpose()
    }

    #[tracing::instrument(skip(self, tx), fields(user_id = id))]
    async fn find_by_id_tx(&self, tx: &mut PgConnection, id: i64) -> Result<Option<User>> {
        let row = sqlx::query(FIND_USER_BY_ID_SQL)
            .bind(id)
            .fetch_optional(tx)
            .await
            .context("failed to query user by id")?;

        row.map(Self::map_user).transpose()
    }

    #[tracing::instrument(skip(self, profile), fields(user_id = id))]
    async fn update_profile(&self, id: i64, profile: UpdateUserProfile) -> Result<User> {
        let row = sqlx::query(UPDATE_USER_PROFILE_SQL)
            .bind(id)
            .bind(profile.primary_email)
            .bind(profile.email_verified)
            .bind(profile.display_name)
            .bind(profile.avatar_url)
            .bind(profile.last_login_at)
            .fetch_one(&self.pool)
            .await
            .context("failed to update user profile")?;

        Self::map_user(row)
    }

    #[tracing::instrument(skip(self, tx, profile), fields(user_id = id))]
    async fn update_profile_tx(
        &self,
        tx: &mut PgConnection,
        id: i64,
        profile: UpdateUserProfile,
    ) -> Result<User> {
        let row = sqlx::query(UPDATE_USER_PROFILE_SQL)
            .bind(id)
            .bind(profile.primary_email)
            .bind(profile.email_verified)
            .bind(profile.display_name)
            .bind(profile.avatar_url)
            .bind(profile.last_login_at)
            .fetch_one(tx)
            .await
            .context("failed to update user profile")?;

        Self::map_user(row)
    }

    async fn touch_last_login(&self, id: i64, last_login_at: DateTime<Utc>) -> Result<()> {
        sqlx::query(TOUCH_USER_LAST_LOGIN_SQL)
            .bind(id)
            .bind(last_login_at)
            .execute(&self.pool)
            .await
            .context("failed to touch users.last_login_at")?;

        Ok(())
    }

    async fn touch_last_login_tx(
        &self,
        tx: &mut PgConnection,
        id: i64,
        last_login_at: DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(TOUCH_USER_LAST_LOGIN_SQL)
            .bind(id)
            .bind(last_login_at)
            .execute(tx)
            .await
            .context("failed to touch users.last_login_at")?;

        Ok(())
    }
}
