use serde::{Deserialize, Serialize};

use crate::domain::entity::user::User;

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserDto {
    pub id: i64,
    pub primary_email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub account_status: &'static str,
}

impl From<User> for UserDto {
    fn from(value: User) -> Self {
        Self {
            id: value.id,
            primary_email: value.primary_email,
            email_verified: value.email_verified,
            display_name: value.display_name,
            avatar_url: value.avatar_url,
            account_status: value.account_status.as_str(),
        }
    }
}
