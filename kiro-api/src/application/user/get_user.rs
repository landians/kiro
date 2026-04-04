use anyhow::Result;

use super::UserLogicError;
use crate::domain::{entity::user::User, repository::user_repository::UserRepository};

pub struct GetUserLogic<UR> {
    user_repository: UR,
}

impl<UR> GetUserLogic<UR>
where
    UR: UserRepository,
{
    pub fn new(user_repository: UR) -> Self {
        Self { user_repository }
    }

    pub async fn execute(&self, user_id: i64) -> Result<User> {
        let Some(user) = self.user_repository.find_by_id(user_id).await? else {
            return Err(UserLogicError::UserNotFound { user_id }.into());
        };

        Ok(user)
    }
}
