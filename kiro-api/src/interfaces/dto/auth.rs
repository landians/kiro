use serde::{Deserialize, Serialize};
use validator::Validate;

use super::user::UserDto;

#[derive(Debug, Deserialize, Validate)]
pub struct GoogleLoginRequest {
    #[validate(custom(function = "validate_non_blank_code"))]
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct GoogleLoginResponse {
    pub user: UserDto,
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub refresh_expires_in: i64,
}

#[derive(Debug, Serialize)]
pub struct RefreshAccessTokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}

fn validate_non_blank_code(code: &str) -> Result<(), validator::ValidationError> {
    if code.trim().is_empty() {
        let mut error = validator::ValidationError::new("blank");
        error.message = Some("code cannot be empty".into());
        return Err(error);
    }

    Ok(())
}
