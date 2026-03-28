#![allow(dead_code)]

use anyhow::{Context, Result};
use sqlx::{PgConnection, Row, postgres::PgRow};

use crate::domain::{
    entity::user_auth_identity::{AuthProvider, UserAuthIdentity},
    repository::user_auth_identity_repository::{
        CreateUserAuthIdentity, UpdateUserAuthIdentitySnapshot,
        UserAuthIdentityRepository as UserAuthIdentityRepositoryTrait,
    },
};

#[derive(Clone, Default)]
pub struct UserAuthIdentityRepository;

impl UserAuthIdentityRepository {
    pub fn new() -> Self {
        Self
    }

    fn map_user_auth_identity(row: PgRow) -> Result<UserAuthIdentity> {
        let provider = row
            .try_get::<String, _>("provider")
            .context("failed to decode user_auth_identities.provider")?;

        Ok(UserAuthIdentity {
            id: row
                .try_get("id")
                .context("failed to decode user_auth_identities.id")?,
            user_id: row
                .try_get("user_id")
                .context("failed to decode user_auth_identities.user_id")?,
            provider: AuthProvider::from_db(&provider)?,
            provider_user_id: row
                .try_get("provider_user_id")
                .context("failed to decode user_auth_identities.provider_user_id")?,
            provider_email: row
                .try_get("provider_email")
                .context("failed to decode user_auth_identities.provider_email")?,
            provider_email_verified: row
                .try_get("provider_email_verified")
                .context("failed to decode user_auth_identities.provider_email_verified")?,
            provider_display_name: row
                .try_get("provider_display_name")
                .context("failed to decode user_auth_identities.provider_display_name")?,
            provider_avatar_url: row
                .try_get("provider_avatar_url")
                .context("failed to decode user_auth_identities.provider_avatar_url")?,
            last_login_at: row
                .try_get("last_login_at")
                .context("failed to decode user_auth_identities.last_login_at")?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode user_auth_identities.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode user_auth_identities.updated_at")?,
        })
    }
}

impl UserAuthIdentityRepositoryTrait for UserAuthIdentityRepository {
    async fn create(
        &self,
        conn: &mut PgConnection,
        identity: CreateUserAuthIdentity,
    ) -> Result<UserAuthIdentity> {
        let row = sqlx::query(
            r#"
            insert into user_auth_identities (
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_verified,
                provider_display_name,
                provider_avatar_url,
                last_login_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning
                id,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_verified,
                provider_display_name,
                provider_avatar_url,
                last_login_at,
                created_at,
                updated_at
            "#,
        )
        .bind(identity.user_id)
        .bind(identity.provider.as_str())
        .bind(identity.provider_user_id)
        .bind(identity.provider_email)
        .bind(identity.provider_email_verified)
        .bind(identity.provider_display_name)
        .bind(identity.provider_avatar_url)
        .bind(identity.last_login_at)
        .fetch_one(conn)
        .await
        .context("failed to insert user auth identity")?;

        Self::map_user_auth_identity(row)
    }

    async fn find_by_provider_user_id(
        &self,
        conn: &mut PgConnection,
        provider: AuthProvider,
        provider_user_id: &str,
    ) -> Result<Option<UserAuthIdentity>> {
        let row = sqlx::query(
            r#"
            select
                id,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_verified,
                provider_display_name,
                provider_avatar_url,
                last_login_at,
                created_at,
                updated_at
            from user_auth_identities
            where provider = $1 and provider_user_id = $2
            "#,
        )
        .bind(provider.as_str())
        .bind(provider_user_id)
        .fetch_optional(conn)
        .await
        .context("failed to query user auth identity by provider user id")?;

        row.map(Self::map_user_auth_identity).transpose()
    }

    async fn update_snapshot(
        &self,
        conn: &mut PgConnection,
        id: i64,
        snapshot: UpdateUserAuthIdentitySnapshot,
    ) -> Result<UserAuthIdentity> {
        let row = sqlx::query(
            r#"
            update user_auth_identities
            set
                provider_email = $2,
                provider_email_verified = $3,
                provider_display_name = $4,
                provider_avatar_url = $5,
                last_login_at = $6,
                updated_at = now()
            where id = $1
            returning
                id,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_verified,
                provider_display_name,
                provider_avatar_url,
                last_login_at,
                created_at,
                updated_at
            "#,
        )
        .bind(id)
        .bind(snapshot.provider_email)
        .bind(snapshot.provider_email_verified)
        .bind(snapshot.provider_display_name)
        .bind(snapshot.provider_avatar_url)
        .bind(snapshot.last_login_at)
        .fetch_one(conn)
        .await
        .context("failed to update user auth identity snapshot")?;

        Self::map_user_auth_identity(row)
    }
}
