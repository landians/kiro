#![allow(dead_code)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgConnection, PgPool};
use thiserror::Error;

use crate::domain::{
    entity::{
        user::User,
        user_auth_identity::{AuthProvider, UserAuthIdentity},
    },
    repository::{
        user_auth_identity_repository::{
            CreateUserAuthIdentity, UpdateUserAuthIdentitySnapshot, UserAuthIdentityRepository,
        },
        user_repository::{CreateUser, UpdateUserProfile, UserRepository},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoogleLogin {
    pub provider_user_id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub login_at: DateTime<Utc>,
}

pub struct GoogleLoginLogic<UR, IR> {
    pool: PgPool,
    user_repository: UR,
    user_auth_identity_repository: IR,
}

impl<UR, IR> GoogleLoginLogic<UR, IR>
where
    UR: UserRepository,
    IR: UserAuthIdentityRepository,
{
    pub fn new(pool: PgPool, user_repository: UR, user_auth_identity_repository: IR) -> Self {
        Self {
            pool,
            user_repository,
            user_auth_identity_repository,
        }
    }

    pub async fn execute(&self, login: GoogleLogin) -> Result<User> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("failed to begin google login transaction")?;

        let identity = self
            .user_auth_identity_repository
            .find_by_provider_user_id_tx(tx.as_mut(), AuthProvider::Google, &login.provider_user_id)
            .await
            .context("failed to load google user auth identity")?;

        let user = match identity {
            Some(identity) => {
                self.login_existing_user(tx.as_mut(), &login, &identity)
                    .await?
            }
            None => self.create_user_and_identity(tx.as_mut(), &login).await?,
        };

        tx.commit()
            .await
            .context("failed to commit google login transaction")?;

        Ok(user)
    }

    async fn login_existing_user(
        &self,
        conn: &mut PgConnection,
        login: &GoogleLogin,
        identity: &UserAuthIdentity,
    ) -> Result<User> {
        let user = self.load_existing_user(conn, identity).await?;
        self.ensure_user_can_login(&user)?;

        self.user_auth_identity_repository
            .update_snapshot_tx(conn, identity.id, self.build_identity_snapshot(login))
            .await
            .context("failed to update google user auth identity snapshot")?;

        self.user_repository
            .update_profile_tx(conn, user.id, self.build_user_profile(login))
            .await
            .context("failed to update user profile during google login")
    }

    async fn create_user_and_identity(
        &self,
        conn: &mut PgConnection,
        login: &GoogleLogin,
    ) -> Result<User> {
        let user = self
            .user_repository
            .create_tx(conn, self.build_create_user(login))
            .await
            .context("failed to create user during google login")?;

        self.user_auth_identity_repository
            .create_tx(conn, self.build_create_identity(user.id, login))
            .await
            .context("failed to create google user auth identity")?;

        Ok(user)
    }

    async fn load_existing_user(
        &self,
        conn: &mut PgConnection,
        identity: &UserAuthIdentity,
    ) -> Result<User> {
        let user = self
            .user_repository
            .find_by_id_tx(conn, identity.user_id)
            .await
            .context("failed to load user linked to google auth identity")?;

        let Some(user) = user else {
            return Err(GoogleLoginError::MissingLinkedUser {
                user_id: identity.user_id,
                identity_id: identity.id,
            }
            .into());
        };

        Ok(user)
    }

    fn ensure_user_can_login(&self, user: &User) -> Result<()> {
        if user.is_active() {
            return Ok(());
        }

        if user.is_frozen() {
            return Err(GoogleLoginError::UserFrozen { user_id: user.id }.into());
        }

        if user.is_banned() {
            return Err(GoogleLoginError::UserBanned { user_id: user.id }.into());
        }

        Err(GoogleLoginError::UserBanned { user_id: user.id }.into())
    }

    fn build_create_user(&self, login: &GoogleLogin) -> CreateUser {
        CreateUser {
            primary_email: login.email.clone(),
            email_verified: login.email_verified,
            display_name: login.display_name.clone(),
            avatar_url: login.avatar_url.clone(),
            last_login_at: Some(login.login_at),
        }
    }

    fn build_user_profile(&self, login: &GoogleLogin) -> UpdateUserProfile {
        UpdateUserProfile {
            primary_email: login.email.clone(),
            email_verified: login.email_verified,
            display_name: login.display_name.clone(),
            avatar_url: login.avatar_url.clone(),
            last_login_at: login.login_at,
        }
    }

    fn build_create_identity(&self, user_id: i64, login: &GoogleLogin) -> CreateUserAuthIdentity {
        CreateUserAuthIdentity {
            user_id,
            provider: AuthProvider::Google,
            provider_user_id: login.provider_user_id.clone(),
            provider_email: login.email.clone(),
            provider_email_verified: login.email_verified,
            provider_display_name: login.display_name.clone(),
            provider_avatar_url: login.avatar_url.clone(),
            last_login_at: Some(login.login_at),
        }
    }

    fn build_identity_snapshot(&self, login: &GoogleLogin) -> UpdateUserAuthIdentitySnapshot {
        UpdateUserAuthIdentitySnapshot {
            provider_email: login.email.clone(),
            provider_email_verified: login.email_verified,
            provider_display_name: login.display_name.clone(),
            provider_avatar_url: login.avatar_url.clone(),
            last_login_at: login.login_at,
        }
    }
}

#[derive(Debug, Error)]
pub enum GoogleLoginError {
    #[error("linked user {user_id} is missing for auth identity {identity_id}")]
    MissingLinkedUser { user_id: i64, identity_id: i64 },
    #[error("user {user_id} is frozen")]
    UserFrozen { user_id: i64 },
    #[error("user {user_id} is banned")]
    UserBanned { user_id: i64 },
}
