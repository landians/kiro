use std::str::FromStr;

use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::domain::repository::user_identity::{
    UserIdentityRepository, UserIdentityRepositoryError,
};
use crate::domain::user_identity::{IdentityProvider, NewUserIdentity, UserIdentity};

#[derive(Clone)]
pub struct PostgresUserIdentityRepository {
    pool: PgPool,
}

impl PostgresUserIdentityRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserIdentityRepository for PostgresUserIdentityRepository {
    async fn create(
        &self,
        new_identity: NewUserIdentity,
    ) -> Result<UserIdentity, UserIdentityRepositoryError> {
        new_identity.validate()?;

        let row = sqlx::query(
            r#"
            insert into kiro.user_identities (
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning
                id,
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at,
                created_at,
                updated_at
            "#,
        )
        .bind(new_identity.identity_code)
        .bind(new_identity.user_id)
        .bind(new_identity.provider.as_str())
        .bind(new_identity.provider_user_id)
        .bind(new_identity.provider_email)
        .bind(new_identity.provider_email_normalized)
        .bind(new_identity.profile)
        .bind(new_identity.last_authenticated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(map_write_error)?;

        map_user_identity(row)
    }

    async fn find_by_id(
        &self,
        identity_id: i64,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        let row = sqlx::query(select_user_identity_sql())
            .bind(identity_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_read_error)?;

        row.map(map_user_identity).transpose()
    }

    async fn find_by_identity_code<'a>(
        &'a self,
        identity_code: &'a str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        let row = sqlx::query(
            r#"
            select
                id,
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at,
                created_at,
                updated_at
            from kiro.user_identities
            where identity_code = $1
            "#,
        )
        .bind(identity_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_read_error)?;

        row.map(map_user_identity).transpose()
    }

    async fn find_by_provider_subject<'a>(
        &'a self,
        provider: IdentityProvider,
        provider_user_id: &'a str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        let row = sqlx::query(
            r#"
            select
                id,
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at,
                created_at,
                updated_at
            from kiro.user_identities
            where provider = $1 and provider_user_id = $2
            "#,
        )
        .bind(provider.as_str())
        .bind(provider_user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_read_error)?;

        row.map(map_user_identity).transpose()
    }

    async fn find_by_provider_email_normalized<'a>(
        &'a self,
        provider: IdentityProvider,
        provider_email_normalized: &'a str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        let row = sqlx::query(
            r#"
            select
                id,
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at,
                created_at,
                updated_at
            from kiro.user_identities
            where provider = $1 and provider_email_normalized = $2
            "#,
        )
        .bind(provider.as_str())
        .bind(provider_email_normalized.trim().to_ascii_lowercase())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_read_error)?;

        row.map(map_user_identity).transpose()
    }

    async fn list_by_user_id(
        &self,
        user_id: i64,
    ) -> Result<Vec<UserIdentity>, UserIdentityRepositoryError> {
        let rows = sqlx::query(
            r#"
            select
                id,
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at,
                created_at,
                updated_at
            from kiro.user_identities
            where user_id = $1
            order by id asc
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_read_error)?;

        rows.into_iter().map(map_user_identity).collect()
    }

    async fn update_last_authenticated_at(
        &self,
        identity_id: i64,
        last_authenticated_at: time::OffsetDateTime,
    ) -> Result<UserIdentity, UserIdentityRepositoryError> {
        let row = sqlx::query(
            r#"
            update kiro.user_identities
            set
                last_authenticated_at = $2,
                updated_at = now()
            where id = $1
            returning
                id,
                identity_code,
                user_id,
                provider,
                provider_user_id,
                provider_email,
                provider_email_normalized,
                profile_jsonb,
                last_authenticated_at,
                created_at,
                updated_at
            "#,
        )
        .bind(identity_id)
        .bind(last_authenticated_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_write_error)?
        .ok_or(UserIdentityRepositoryError::NotFound)?;

        map_user_identity(row)
    }
}

fn select_user_identity_sql() -> &'static str {
    r#"
    select
        id,
        identity_code,
        user_id,
        provider,
        provider_user_id,
        provider_email,
        provider_email_normalized,
        profile_jsonb,
        last_authenticated_at,
        created_at,
        updated_at
    from kiro.user_identities
    where id = $1
    "#
}

fn map_user_identity(row: PgRow) -> Result<UserIdentity, UserIdentityRepositoryError> {
    let provider = row
        .try_get::<String, _>("provider")
        .map_err(map_row_error)
        .and_then(|value| {
            IdentityProvider::from_str(&value).map_err(UserIdentityRepositoryError::from)
        })?;

    Ok(UserIdentity {
        id: row.try_get("id").map_err(map_row_error)?,
        identity_code: row.try_get("identity_code").map_err(map_row_error)?,
        user_id: row.try_get("user_id").map_err(map_row_error)?,
        provider,
        provider_user_id: row.try_get("provider_user_id").map_err(map_row_error)?,
        provider_email: row.try_get("provider_email").map_err(map_row_error)?,
        provider_email_normalized: row
            .try_get("provider_email_normalized")
            .map_err(map_row_error)?,
        profile: row.try_get("profile_jsonb").map_err(map_row_error)?,
        last_authenticated_at: row
            .try_get("last_authenticated_at")
            .map_err(map_row_error)?,
        created_at: row.try_get("created_at").map_err(map_row_error)?,
        updated_at: row.try_get("updated_at").map_err(map_row_error)?,
    })
}

fn map_read_error(error: sqlx::Error) -> UserIdentityRepositoryError {
    UserIdentityRepositoryError::unexpected(format!(
        "failed to query user identity repository: {error}"
    ))
}

fn map_row_error(error: sqlx::Error) -> UserIdentityRepositoryError {
    UserIdentityRepositoryError::unexpected(format!("failed to decode user identity row: {error}"))
}

fn map_write_error(error: sqlx::Error) -> UserIdentityRepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        match database_error.constraint() {
            Some("uk_user_identities_code") => {
                return UserIdentityRepositoryError::Conflict {
                    field: "identity_code",
                };
            }
            Some("uk_user_identities_provider_uid") => {
                return UserIdentityRepositoryError::Conflict {
                    field: "provider_user_id",
                };
            }
            _ => {}
        }
    }

    UserIdentityRepositoryError::unexpected(format!(
        "failed to write user identity repository: {error}"
    ))
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::OnceLock;

    use serde_json::json;
    use sqlx::Row;

    use super::PostgresUserIdentityRepository;
    use crate::config::PostgresConfig;
    use crate::domain::repository::user_identity::{
        UserIdentityRepository, UserIdentityRepositoryError,
    };
    use crate::domain::user_identity::{IdentityProvider, NewUserIdentity};
    use crate::infrastructure::persistence::postgres::{PostgresPoolBuilder, run_migrations};

    static TEST_DATABASE_URL: OnceLock<Option<String>> = OnceLock::new();

    fn test_database_url() -> Option<String> {
        TEST_DATABASE_URL
            .get_or_init(|| {
                env::var("KIRO_TEST_POSTGRES_URL")
                    .ok()
                    .or_else(|| env::var("POSTGRES_URL").ok())
                    .or_else(|| Some("postgres://postgres:postgres@127.0.0.1:5432/kiro".to_owned()))
            })
            .clone()
    }

    async fn test_repository() -> Option<PostgresUserIdentityRepository> {
        let url = test_database_url()?;
        let config = PostgresConfig {
            url,
            max_connections: 1,
            min_connections: 1,
            connect_timeout_seconds: 1,
            acquire_timeout_seconds: 1,
            run_migrations: true,
        };

        let pool = match PostgresPoolBuilder::new(config).build().await {
            Ok(pool) => pool,
            Err(_) => return None,
        };

        if run_migrations(&pool).await.is_err() {
            return None;
        }

        Some(PostgresUserIdentityRepository::new(pool))
    }

    fn unique_suffix(prefix: &str) -> String {
        format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
    }

    async fn insert_test_user(repository: &PostgresUserIdentityRepository, user_code: &str) -> i64 {
        sqlx::query(
            r#"
            insert into kiro.users (
                user_code,
                email,
                email_normalized,
                locale,
                time_zone,
                status
            )
            values ($1, $2, $3, 'en-US', 'UTC', 'active')
            returning id
            "#,
        )
        .bind(user_code)
        .bind(format!("{user_code}@example.com"))
        .bind(format!("{user_code}@example.com"))
        .fetch_one(&repository.pool)
        .await
        .expect("test user should insert")
        .get::<i64, _>("id")
    }

    async fn delete_test_user(repository: &PostgresUserIdentityRepository, user_code: &str) {
        let _ = sqlx::query("delete from kiro.users where user_code = $1")
            .bind(user_code)
            .execute(&repository.pool)
            .await;
    }

    #[tokio::test]
    async fn create_and_find_identity_round_trip() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_suffix("m12_identity_user");
        let user_id = insert_test_user(&repository, &user_code).await;
        let identity_code = unique_suffix("m12_identity");
        let provider_user_id = unique_suffix("google-user");

        let created = repository
            .create(
                NewUserIdentity::new(
                    identity_code.clone(),
                    user_id,
                    IdentityProvider::Google,
                    provider_user_id.clone(),
                )
                .with_provider_email("Identity.User@example.com")
                .with_profile(json!({"sub": provider_user_id})),
            )
            .await
            .expect("identity should be created");

        let found_by_subject = repository
            .find_by_provider_subject(IdentityProvider::Google, &provider_user_id)
            .await
            .expect("provider subject lookup should succeed")
            .expect("identity should exist");

        assert_eq!(found_by_subject.id, created.id);

        let found_by_email = repository
            .find_by_provider_email_normalized(
                IdentityProvider::Google,
                "  identity.user@example.com ",
            )
            .await
            .expect("provider email lookup should succeed")
            .expect("identity should exist");

        assert_eq!(found_by_email.id, created.id);

        delete_test_user(&repository, &user_code).await;
    }

    #[tokio::test]
    async fn list_identities_by_user_id_returns_all_bindings() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_suffix("m12_identity_list_user");
        let user_id = insert_test_user(&repository, &user_code).await;
        let first_identity_code = unique_suffix("m12_identity_first");
        let second_identity_code = unique_suffix("m12_identity_second");

        repository
            .create(NewUserIdentity::new(
                first_identity_code,
                user_id,
                IdentityProvider::Google,
                unique_suffix("google-subject-first"),
            ))
            .await
            .expect("first identity should be created");

        repository
            .create(
                NewUserIdentity::new(
                    second_identity_code,
                    user_id,
                    IdentityProvider::Google,
                    unique_suffix("google-subject-second"),
                )
                .with_provider_email("list@example.com"),
            )
            .await
            .expect("second identity should be created");

        let identities = repository
            .list_by_user_id(user_id)
            .await
            .expect("list identities should succeed");

        assert_eq!(identities.len(), 2);

        delete_test_user(&repository, &user_code).await;
    }

    #[tokio::test]
    async fn update_last_authenticated_at_persists_timestamp() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_suffix("m12_identity_auth_user");
        let user_id = insert_test_user(&repository, &user_code).await;
        let identity = repository
            .create(NewUserIdentity::new(
                unique_suffix("m12_identity_auth"),
                user_id,
                IdentityProvider::Google,
                unique_suffix("google-subject-auth"),
            ))
            .await
            .expect("identity should be created");

        let authenticated_at = time::OffsetDateTime::now_utc();
        let updated = repository
            .update_last_authenticated_at(identity.id, authenticated_at)
            .await
            .expect("update last authenticated should succeed");

        assert_eq!(updated.last_authenticated_at, Some(authenticated_at));

        delete_test_user(&repository, &user_code).await;
    }

    #[tokio::test]
    async fn duplicate_provider_subject_is_rejected() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_suffix("m12_identity_conflict_user");
        let user_id = insert_test_user(&repository, &user_code).await;
        let provider_user_id = unique_suffix("google-subject-conflict");

        repository
            .create(NewUserIdentity::new(
                unique_suffix("m12_identity_conflict_first"),
                user_id,
                IdentityProvider::Google,
                provider_user_id.clone(),
            ))
            .await
            .expect("first identity should be created");

        let error = repository
            .create(NewUserIdentity::new(
                unique_suffix("m12_identity_conflict_second"),
                user_id,
                IdentityProvider::Google,
                provider_user_id,
            ))
            .await
            .expect_err("duplicate provider subject should fail");

        assert_eq!(
            error,
            UserIdentityRepositoryError::Conflict {
                field: "provider_user_id",
            }
        );

        delete_test_user(&repository, &user_code).await;
    }
}
