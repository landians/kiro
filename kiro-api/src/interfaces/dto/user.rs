use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::domain::entity::user::User;

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_update_user_request"))]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    #[validate(url)]
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

fn validate_update_user_request(
    request: &UpdateUserRequest,
) -> Result<(), validator::ValidationError> {
    if request.display_name.is_none() && request.avatar_url.is_none() {
        let mut error = validator::ValidationError::new("empty_user_update");
        error.message = Some("at least one updatable field is required".into());
        return Err(error);
    }

    Ok(())
}
