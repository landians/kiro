use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::domain::entity::user_auth_identity::{AuthProvider, UserAuthIdentity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateUserAuthIdentity {
    pub user_id: i64,
    pub provider: AuthProvider,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub provider_email_verified: bool,
    pub provider_display_name: Option<String>,
    pub provider_avatar_url: Option<String>,
    pub provider_profile: serde_json::Value,
    pub last_login_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUserAuthIdentitySnapshot {
    pub provider_email: Option<String>,
    pub provider_email_verified: bool,
    pub provider_display_name: Option<String>,
    pub provider_avatar_url: Option<String>,
    pub provider_profile: serde_json::Value,
    pub last_login_at: DateTime<Utc>,
}

pub trait UserAuthIdentityRepository: Send + Sync {
    async fn create(&self, identity: CreateUserAuthIdentity) -> Result<UserAuthIdentity>;

    async fn find_by_provider_user_id(
        &self,
        provider: AuthProvider,
        provider_user_id: &str,
    ) -> Result<Option<UserAuthIdentity>>;

    async fn update_snapshot(
        &self,
        id: i64,
        snapshot: UpdateUserAuthIdentitySnapshot,
    ) -> Result<UserAuthIdentity>;
}
