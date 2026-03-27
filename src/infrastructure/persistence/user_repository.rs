use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{
    entity::user::{AccountStatus, User},
    repository::user_repository::{
        CreateUser,
        UpdateUserProfile,
        UserRepository as UserRepositoryTrait,
    },
};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[allow(dead_code)]
    fn build_user(
        &self,
        id: i64,
        primary_email: Option<String>,
        email_verified: bool,
        display_name: Option<String>,
        avatar_url: Option<String>,
        account_status: AccountStatus,
        frozen_at: Option<DateTime<Utc>>,
        banned_at: Option<DateTime<Utc>>,
        last_login_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> User {
        User {
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
            updated_at,
        }
    }
}

impl UserRepositoryTrait for UserRepository {
    async fn create(&self, _user: CreateUser) -> Result<User> {
        bail!("UserRepository::create is not implemented yet")
    }

    async fn find_by_id(&self, _id: i64) -> Result<Option<User>> {
        bail!("UserRepository::find_by_id is not implemented yet")
    }

    async fn update_profile(&self, _id: i64, _profile: UpdateUserProfile) -> Result<User> {
        bail!("UserRepository::update_profile is not implemented yet")
    }

    async fn touch_last_login(&self, _id: i64, _last_login_at: DateTime<Utc>) -> Result<()> {
        bail!("UserRepository::touch_last_login is not implemented yet")
    }
}
