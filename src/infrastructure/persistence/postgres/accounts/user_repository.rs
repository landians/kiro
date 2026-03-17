use std::str::FromStr;

use sqlx::PgPool;
use sqlx::Row;
use sqlx::postgres::PgRow;

use crate::domain::account::{NewUser, User, UserStatus};
use crate::domain::repository::user::{UserRepository, UserRepositoryError};

#[derive(Clone)]
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserRepository for PostgresUserRepository {
    async fn create(&self, new_user: NewUser) -> Result<User, UserRepositoryError> {
        new_user.validate()?;

        let row = sqlx::query(
            r#"
            insert into kiro.users (
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning
                id,
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status,
                last_login_at,
                created_at,
                updated_at
            "#,
        )
        .bind(new_user.user_code)
        .bind(new_user.email)
        .bind(new_user.email_normalized)
        .bind(new_user.display_name)
        .bind(new_user.avatar_url)
        .bind(new_user.locale)
        .bind(new_user.time_zone)
        .bind(new_user.status.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_write_error)?;

        map_user(row)
    }

    async fn find_by_id(&self, user_id: i64) -> Result<Option<User>, UserRepositoryError> {
        let row = sqlx::query(
            r#"
            select
                id,
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status,
                last_login_at,
                created_at,
                updated_at
            from kiro.users
            where id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_read_error)?;

        row.map(map_user).transpose()
    }

    async fn find_by_user_code<'a>(
        &'a self,
        user_code: &'a str,
    ) -> Result<Option<User>, UserRepositoryError> {
        let row = sqlx::query(
            r#"
            select
                id,
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status,
                last_login_at,
                created_at,
                updated_at
            from kiro.users
            where user_code = $1
            "#,
        )
        .bind(user_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_read_error)?;

        row.map(map_user).transpose()
    }

    async fn find_by_email_normalized<'a>(
        &'a self,
        email_normalized: &'a str,
    ) -> Result<Option<User>, UserRepositoryError> {
        let row = sqlx::query(
            r#"
            select
                id,
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status,
                last_login_at,
                created_at,
                updated_at
            from kiro.users
            where email_normalized = $1
            "#,
        )
        .bind(email_normalized.trim().to_ascii_lowercase())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_read_error)?;

        row.map(map_user).transpose()
    }

    async fn update_status(
        &self,
        user_id: i64,
        next_status: UserStatus,
    ) -> Result<User, UserRepositoryError> {
        let mut current_user = self
            .find_by_id(user_id)
            .await?
            .ok_or(UserRepositoryError::NotFound)?;

        if current_user.status == next_status {
            return Ok(current_user);
        }

        current_user.transition_to(next_status)?;

        let row = sqlx::query(
            r#"
            update kiro.users
            set
                status = $2,
                updated_at = now()
            where id = $1
            returning
                id,
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status,
                last_login_at,
                created_at,
                updated_at
            "#,
        )
        .bind(user_id)
        .bind(next_status.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(map_write_error)?;

        map_user(row)
    }

    async fn update_last_login_at(
        &self,
        user_id: i64,
        last_login_at: time::OffsetDateTime,
    ) -> Result<User, UserRepositoryError> {
        let row = sqlx::query(
            r#"
            update kiro.users
            set
                last_login_at = $2,
                updated_at = now()
            where id = $1
            returning
                id,
                user_code,
                email,
                email_normalized,
                display_name,
                avatar_url,
                locale,
                time_zone,
                status,
                last_login_at,
                created_at,
                updated_at
            "#,
        )
        .bind(user_id)
        .bind(last_login_at)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_write_error)?
        .ok_or(UserRepositoryError::NotFound)?;

        map_user(row)
    }
}

fn map_user(row: PgRow) -> Result<User, UserRepositoryError> {
    let status = row
        .try_get::<String, _>("status")
        .map_err(map_row_error)
        .and_then(|value| UserStatus::from_str(&value).map_err(UserRepositoryError::from))?;

    Ok(User {
        id: row.try_get("id").map_err(map_row_error)?,
        user_code: row.try_get("user_code").map_err(map_row_error)?,
        email: row.try_get("email").map_err(map_row_error)?,
        email_normalized: row.try_get("email_normalized").map_err(map_row_error)?,
        display_name: row.try_get("display_name").map_err(map_row_error)?,
        avatar_url: row.try_get("avatar_url").map_err(map_row_error)?,
        locale: row.try_get("locale").map_err(map_row_error)?,
        time_zone: row.try_get("time_zone").map_err(map_row_error)?,
        status,
        last_login_at: row.try_get("last_login_at").map_err(map_row_error)?,
        created_at: row.try_get("created_at").map_err(map_row_error)?,
        updated_at: row.try_get("updated_at").map_err(map_row_error)?,
    })
}

fn map_read_error(error: sqlx::Error) -> UserRepositoryError {
    UserRepositoryError::unexpected(format!("failed to query user repository: {error}"))
}

fn map_row_error(error: sqlx::Error) -> UserRepositoryError {
    UserRepositoryError::unexpected(format!("failed to decode user row: {error}"))
}

fn map_write_error(error: sqlx::Error) -> UserRepositoryError {
    if let sqlx::Error::Database(database_error) = &error {
        match database_error.constraint() {
            Some("uk_users_user_code") => {
                return UserRepositoryError::Conflict { field: "user_code" };
            }
            Some("uk_users_email_norm") => {
                return UserRepositoryError::Conflict {
                    field: "email_normalized",
                };
            }
            _ => {}
        }
    }

    UserRepositoryError::unexpected(format!("failed to write user repository: {error}"))
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::sync::OnceLock;

    use super::PostgresUserRepository;
    use crate::config::PostgresConfig;
    use crate::domain::account::{NewUser, UserStatus};
    use crate::domain::repository::user::{UserRepository, UserRepositoryError};
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

    async fn test_repository() -> Option<PostgresUserRepository> {
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

        let repository = PostgresUserRepository::new(pool);
        Some(repository)
    }

    fn unique_user_code(prefix: &str) -> String {
        format!("{prefix}_{}", uuid::Uuid::new_v4().simple())
    }

    async fn delete_user_by_code(repository: &PostgresUserRepository, user_code: &str) {
        let _ = sqlx::query("delete from kiro.users where user_code = $1")
            .bind(user_code)
            .execute(&repository.pool)
            .await;
    }

    #[tokio::test]
    async fn create_and_find_user_round_trip() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_user_code("m11_create_find");
        delete_user_by_code(&repository, &user_code).await;

        let new_user = NewUser::new(user_code.clone())
            .with_email("M1.1.User@example.com")
            .with_display_name("M1.1 User");
        let created = repository
            .create(new_user)
            .await
            .expect("user should be created");

        let found = repository
            .find_by_id(created.id)
            .await
            .expect("user query should succeed")
            .expect("user should exist");

        assert_eq!(found.user_code, user_code);
        assert_eq!(
            found.email_normalized.as_deref(),
            Some("m1.1.user@example.com")
        );
        assert_eq!(found.status, UserStatus::Pending);

        let found_by_email = repository
            .find_by_email_normalized("  M1.1.USER@example.com ")
            .await
            .expect("email lookup should succeed")
            .expect("user should exist");

        assert_eq!(found_by_email.id, created.id);

        delete_user_by_code(&repository, &user_code).await;
    }

    #[tokio::test]
    async fn update_status_persists_expected_transition() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_user_code("m11_update_status");
        delete_user_by_code(&repository, &user_code).await;

        let new_user = NewUser::new(user_code.clone()).with_status(UserStatus::Active);
        let created = repository
            .create(new_user)
            .await
            .expect("user should be created");

        let updated = repository
            .update_status(created.id, UserStatus::Disabled)
            .await
            .expect("status update should succeed");

        assert_eq!(updated.status, UserStatus::Disabled);

        let reloaded = repository
            .find_by_user_code(&user_code)
            .await
            .expect("reload should succeed")
            .expect("user should exist");

        assert_eq!(reloaded.status, UserStatus::Disabled);

        delete_user_by_code(&repository, &user_code).await;
    }

    #[tokio::test]
    async fn invalid_status_transition_is_rejected() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_user_code("m11_invalid_transition");
        delete_user_by_code(&repository, &user_code).await;

        let new_user = NewUser::new(user_code.clone()).with_status(UserStatus::Active);
        let created = repository
            .create(new_user)
            .await
            .expect("user should be created");

        let deleted = repository
            .update_status(created.id, UserStatus::Deleted)
            .await
            .expect("active -> deleted should succeed");

        assert_eq!(deleted.status, UserStatus::Deleted);

        let error = repository
            .update_status(created.id, UserStatus::Active)
            .await
            .expect_err("deleted -> active should fail");

        assert!(matches!(error, UserRepositoryError::Domain(_)));

        delete_user_by_code(&repository, &user_code).await;
    }

    #[tokio::test]
    async fn update_last_login_at_persists_timestamp() {
        let Some(repository) = test_repository().await else {
            return;
        };

        let user_code = unique_user_code("m11_last_login");
        delete_user_by_code(&repository, &user_code).await;

        let new_user = NewUser::new(user_code.clone()).with_status(UserStatus::Active);
        let created = repository
            .create(new_user)
            .await
            .expect("user should be created");

        let last_login_at = time::OffsetDateTime::now_utc();
        let updated = repository
            .update_last_login_at(created.id, last_login_at)
            .await
            .expect("last login update should succeed");

        assert_eq!(updated.last_login_at, Some(last_login_at));

        let reloaded = repository
            .find_by_id(created.id)
            .await
            .expect("reload should succeed")
            .expect("user should exist");

        assert_eq!(reloaded.last_login_at, Some(last_login_at));

        delete_user_by_code(&repository, &user_code).await;
    }
}
