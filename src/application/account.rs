use time::OffsetDateTime;

use crate::domain::account::{NewUser, User, UserStatus};
use crate::domain::repository::user::{UserRepository, UserRepositoryError};
#[cfg(test)]
use crate::infrastructure::persistence::in_memory::accounts::user_repository::InMemoryUserRepository;
use crate::infrastructure::persistence::postgres::accounts::user_repository::PostgresUserRepository;

pub type DefaultAccountService = AccountService<PostgresUserRepository>;
#[cfg(test)]
pub type TestAccountService = AccountService<InMemoryUserRepository>;

#[derive(Clone)]
pub struct AccountService<R>
where
    R: UserRepository,
{
    user_repository: R,
}

impl<R> AccountService<R>
where
    R: UserRepository,
{
    pub fn new(user_repository: R) -> Self {
        Self { user_repository }
    }

    pub async fn create_user(&self, new_user: NewUser) -> Result<User, UserRepositoryError> {
        self.user_repository.create(new_user).await
    }

    pub async fn find_user_by_id(&self, user_id: i64) -> Result<Option<User>, UserRepositoryError> {
        self.user_repository.find_by_id(user_id).await
    }

    pub async fn find_user_by_user_code(
        &self,
        user_code: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        self.user_repository.find_by_user_code(user_code).await
    }

    pub async fn find_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<User>, UserRepositoryError> {
        let Some(email_normalized) = normalize_email(email) else {
            return Ok(None);
        };

        self.user_repository
            .find_by_email_normalized(&email_normalized)
            .await
    }

    pub async fn activate_user(&self, user_id: i64) -> Result<User, UserRepositoryError> {
        self.user_repository
            .update_status(user_id, UserStatus::Active)
            .await
    }

    pub async fn disable_user(&self, user_id: i64) -> Result<User, UserRepositoryError> {
        self.user_repository
            .update_status(user_id, UserStatus::Disabled)
            .await
    }

    pub async fn restore_user(&self, user_id: i64) -> Result<User, UserRepositoryError> {
        self.user_repository
            .update_status(user_id, UserStatus::Active)
            .await
    }

    pub async fn record_last_login(&self, user_id: i64) -> Result<User, UserRepositoryError> {
        self.user_repository
            .update_last_login_at(user_id, OffsetDateTime::now_utc())
            .await
    }
}

fn normalize_email(email: &str) -> Option<String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use time::OffsetDateTime;

    use super::AccountService;
    use crate::domain::account::{NewUser, User, UserStatus};
    use crate::domain::repository::user::{UserRepository, UserRepositoryError};

    #[derive(Clone, Default)]
    struct TestUserRepository {
        users: Arc<Mutex<HashMap<i64, User>>>,
    }

    impl TestUserRepository {
        fn seeded(user: User) -> Self {
            let mut users = HashMap::new();
            users.insert(user.id, user);
            let users = Arc::new(Mutex::new(users));
            Self { users }
        }
    }

    impl UserRepository for TestUserRepository {
        async fn create(&self, new_user: NewUser) -> Result<User, UserRepositoryError> {
            new_user.validate()?;

            let mut users = self.users.lock().expect("users mutex should lock");
            let user = User {
                id: (users.len() + 1) as i64,
                user_code: new_user.user_code,
                email: new_user.email,
                email_normalized: new_user.email_normalized,
                display_name: new_user.display_name,
                avatar_url: new_user.avatar_url,
                locale: new_user.locale,
                time_zone: new_user.time_zone,
                status: new_user.status,
                last_login_at: None,
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            };

            users.insert(user.id, user.clone());
            Ok(user)
        }

        async fn find_by_id(&self, user_id: i64) -> Result<Option<User>, UserRepositoryError> {
            Ok(self
                .users
                .lock()
                .expect("users mutex should lock")
                .get(&user_id)
                .cloned())
        }

        async fn find_by_user_code<'a>(
            &'a self,
            user_code: &'a str,
        ) -> Result<Option<User>, UserRepositoryError> {
            Ok(self
                .users
                .lock()
                .expect("users mutex should lock")
                .values()
                .find(|user| user.user_code == user_code)
                .cloned())
        }

        async fn find_by_email_normalized<'a>(
            &'a self,
            email_normalized: &'a str,
        ) -> Result<Option<User>, UserRepositoryError> {
            Ok(self
                .users
                .lock()
                .expect("users mutex should lock")
                .values()
                .find(|user| user.email_normalized.as_deref() == Some(email_normalized))
                .cloned())
        }

        async fn update_status(
            &self,
            user_id: i64,
            next_status: UserStatus,
        ) -> Result<User, UserRepositoryError> {
            let mut users = self.users.lock().expect("users mutex should lock");
            let user = users
                .get_mut(&user_id)
                .ok_or(UserRepositoryError::NotFound)?;
            user.transition_to(next_status)?;
            user.updated_at = OffsetDateTime::now_utc();
            Ok(user.clone())
        }

        async fn update_last_login_at(
            &self,
            user_id: i64,
            last_login_at: OffsetDateTime,
        ) -> Result<User, UserRepositoryError> {
            let mut users = self.users.lock().expect("users mutex should lock");
            let user = users
                .get_mut(&user_id)
                .ok_or(UserRepositoryError::NotFound)?;
            user.last_login_at = Some(last_login_at);
            user.updated_at = last_login_at;
            Ok(user.clone())
        }
    }

    fn test_user() -> User {
        let now = OffsetDateTime::now_utc();
        User {
            id: 7,
            user_code: "user_7".to_owned(),
            email: Some("hello@example.com".to_owned()),
            email_normalized: Some("hello@example.com".to_owned()),
            display_name: Some("Hello".to_owned()),
            avatar_url: None,
            locale: "en-US".to_owned(),
            time_zone: "UTC".to_owned(),
            status: UserStatus::Pending,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn find_user_by_email_normalizes_input() {
        let user_repository = TestUserRepository::seeded(test_user());
        let service = AccountService::new(user_repository);

        let found = service
            .find_user_by_email("  HELLO@example.com ")
            .await
            .expect("email lookup should succeed")
            .expect("user should exist");

        assert_eq!(found.id, 7);
    }

    #[tokio::test]
    async fn create_find_and_update_status_via_service() {
        let service = AccountService::new(TestUserRepository::default());
        let new_user = NewUser::new("user_new").with_email("new@example.com");

        let created = service
            .create_user(new_user)
            .await
            .expect("user create should succeed");

        let found_by_id = service
            .find_user_by_id(created.id)
            .await
            .expect("find by id should succeed")
            .expect("user should exist");
        assert_eq!(found_by_id.user_code, "user_new");

        let found_by_code = service
            .find_user_by_user_code("user_new")
            .await
            .expect("find by user_code should succeed")
            .expect("user should exist");
        assert_eq!(found_by_code.id, created.id);

        let activated = service
            .activate_user(created.id)
            .await
            .expect("activate should succeed");
        assert_eq!(activated.status, UserStatus::Active);

        let disabled = service
            .disable_user(created.id)
            .await
            .expect("disable should succeed");
        assert_eq!(disabled.status, UserStatus::Disabled);

        let restored = service
            .restore_user(created.id)
            .await
            .expect("restore should succeed");
        assert_eq!(restored.status, UserStatus::Active);
    }

    #[tokio::test]
    async fn record_last_login_updates_timestamp() {
        let user_repository = TestUserRepository::seeded(test_user());
        let service = AccountService::new(user_repository);

        let updated = service
            .record_last_login(7)
            .await
            .expect("last login update should succeed");

        assert!(updated.last_login_at.is_some());
    }
}
