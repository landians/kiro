pub mod get_user;
pub mod update_user;

use anyhow::Result;
use thiserror::Error;

pub use self::update_user::UpdateUser;
use self::{get_user::GetUserLogic, update_user::UpdateUserLogic};
use crate::domain::{entity::user::User, repository::user_repository::UserRepository};

pub struct UserLogic<UR> {
    get_user_logic: GetUserLogic<UR>,
    update_user_logic: UpdateUserLogic<UR>,
}

impl<UR> UserLogic<UR>
where
    UR: UserRepository + Clone,
{
    pub fn new(user_repository: UR) -> Self {
        Self {
            get_user_logic: GetUserLogic::new(user_repository.clone()),
            update_user_logic: UpdateUserLogic::new(user_repository),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get(&self, user_id: i64) -> Result<User> {
        self.get_user_logic.execute(user_id).await
    }

    #[tracing::instrument(skip(self, update), fields(user_id, actor_user_id))]
    pub async fn update(
        &self,
        actor_user_id: i64,
        user_id: i64,
        update: UpdateUser,
    ) -> Result<User> {
        self.update_user_logic
            .execute(actor_user_id, user_id, update)
            .await
    }
}

#[derive(Debug, Error)]
pub enum UserLogicError {
    #[error("user {user_id} not found")]
    UserNotFound { user_id: i64 },
    #[error("user update payload cannot be empty")]
    EmptyUserUpdate,
    #[error("user {actor_user_id} cannot update user {user_id}")]
    UserUpdateForbidden { actor_user_id: i64, user_id: i64 },
}
