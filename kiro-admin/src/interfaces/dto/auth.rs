use serde::{Deserialize, Serialize};

use super::admin_user::AdminUserDto;

#[derive(Debug, Deserialize)]
pub struct PasswordLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct PasswordLoginResponse {
    pub admin_user: AdminUserDto,
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
}
