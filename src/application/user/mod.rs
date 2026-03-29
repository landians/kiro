use anyhow::Result;
use thiserror::Error;

use crate::domain::{entity::user::User, repository::user_repository::UserRepository};

pub struct UserLogic<UR> {
    user_repository: UR,
}

impl<UR> UserLogic<UR>
where
    UR: UserRepository,
{
    pub fn new(user_repository: UR) -> Self {
        Self { user_repository }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get(&self, user_id: i64) -> Result<User> {
        let Some(user) = self.user_repository.find_by_id(user_id).await? else {
            return Err(UserLogicError::UserNotFound { user_id }.into());
        };

        Ok(user)
    }
}

#[derive(Debug, Error)]
pub enum UserLogicError {
    #[error("user {user_id} not found")]
    UserNotFound { user_id: i64 },
}
