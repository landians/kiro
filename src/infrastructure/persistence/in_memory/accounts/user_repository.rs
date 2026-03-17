use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use time::OffsetDateTime;

use crate::domain::account::{NewUser, User, UserStatus};
use crate::domain::repository::user::{UserRepository, UserRepositoryError};

#[derive(Clone, Default)]
pub struct InMemoryUserRepository {
    users: Arc<Mutex<HashMap<i64, User>>>,
}

impl InMemoryUserRepository {
    pub fn seeded(user: User) -> Self {
        let mut users = HashMap::new();
        users.insert(user.id, user);
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

impl UserRepository for InMemoryUserRepository {
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
