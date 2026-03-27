use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::domain::{
    entity::user_auth_identity::{AuthProvider, UserAuthIdentity},
    repository::user_auth_identity_repository::{
        CreateUserAuthIdentity,
        UpdateUserAuthIdentitySnapshot,
        UserAuthIdentityRepository as UserAuthIdentityRepositoryTrait,
    },
};

#[derive(Clone)]
pub struct UserAuthIdentityRepository {
    pool: PgPool,
}

impl UserAuthIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    #[allow(dead_code)]
    fn build_user_auth_identity(
        &self,
        id: i64,
        user_id: i64,
        provider: AuthProvider,
        provider_user_id: String,
        provider_email: Option<String>,
        provider_email_verified: bool,
        provider_display_name: Option<String>,
        provider_avatar_url: Option<String>,
        provider_profile: serde_json::Value,
        last_login_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> UserAuthIdentity {
        UserAuthIdentity {
            id,
            user_id,
            provider,
            provider_user_id,
            provider_email,
            provider_email_verified,
            provider_display_name,
            provider_avatar_url,
            provider_profile,
            last_login_at,
            created_at,
            updated_at,
        }
    }
}

impl UserAuthIdentityRepositoryTrait for UserAuthIdentityRepository {
    async fn create(&self, _identity: CreateUserAuthIdentity) -> Result<UserAuthIdentity> {
        bail!("UserAuthIdentityRepository::create is not implemented yet")
    }

    async fn find_by_provider_user_id(
        &self,
        _provider: AuthProvider,
        _provider_user_id: &str,
    ) -> Result<Option<UserAuthIdentity>> {
        bail!("UserAuthIdentityRepository::find_by_provider_user_id is not implemented yet")
    }

    async fn update_snapshot(
        &self,
        _id: i64,
        _snapshot: UpdateUserAuthIdentitySnapshot,
    ) -> Result<UserAuthIdentity> {
        bail!("UserAuthIdentityRepository::update_snapshot is not implemented yet")
    }
}
