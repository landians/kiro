use anyhow::Result;

use super::UserLogicError;
use crate::domain::{
    entity::user::User,
    repository::user_repository::{UpdateUserProfile, UserRepository},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateUser {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct UpdateUserLogic<UR> {
    user_repository: UR,
}

impl<UR> UpdateUserLogic<UR>
where
    UR: UserRepository,
{
    pub fn new(user_repository: UR) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        actor_user_id: i64,
        user_id: i64,
        update: UpdateUser,
    ) -> Result<User> {
        if actor_user_id != user_id {
            return Err(UserLogicError::UserUpdateForbidden {
                actor_user_id,
                user_id,
            }
            .into());
        }

        if update.display_name.is_none() && update.avatar_url.is_none() {
            return Err(UserLogicError::EmptyUserUpdate.into());
        }

        let Some(user) = self.user_repository.find_by_id(user_id).await? else {
            return Err(UserLogicError::UserNotFound { user_id }.into());
        };

        let profile = UpdateUserProfile {
            primary_email: user.primary_email,
            email_verified: user.email_verified,
            display_name: update
                .display_name
                .map(normalize_optional_text)
                .unwrap_or(user.display_name),
            avatar_url: update
                .avatar_url
                .map(normalize_optional_text)
                .unwrap_or(user.avatar_url),
            last_login_at: user.last_login_at,
        };

        self.user_repository.update_profile(user_id, profile).await
    }
}

fn normalize_optional_text(value: String) -> Option<String> {
    let value = value.trim();

    if value.is_empty() {
        return None;
    }

    Some(value.to_owned())
}
