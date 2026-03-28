use serde::{Deserialize, Serialize};

use super::user::UserDto;

#[derive(Debug, Deserialize)]
pub struct GoogleLoginRequest {
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
