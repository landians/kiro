use serde::{Deserialize, Serialize};

use crate::domain::account::User;

#[derive(Debug, Serialize)]
pub struct ProtectedSessionResponse {
    pub subject: String,
    pub jti: String,
    pub ua_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: u64,
    pub refresh_token_expires_at: u64,
}

#[derive(Debug, Serialize)]
pub struct LogoutSessionResponse {
    pub subject: String,
    pub access_token_revoked: bool,
    pub refresh_token_revoked: bool,
}

#[derive(Debug, Serialize)]
pub struct GoogleAuthorizationUrlResponse {
    pub authorization_url: String,
    pub state: String,
    pub nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GoogleCallbackResponse {
    pub user_code: String,
    pub identity_code: String,
    pub provider: String,
    pub is_new_user: bool,
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: u64,
    pub refresh_token_expires_at: u64,
}

#[derive(Debug, Serialize)]
pub struct CurrentUserResponse {
    pub user_code: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub locale: String,
    pub time_zone: String,
    pub status: String,
    pub last_login_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<User> for CurrentUserResponse {
    fn from(user: User) -> Self {
        Self {
            user_code: user.user_code,
            email: user.email,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            locale: user.locale,
            time_zone: user.time_zone,
            status: user.status.as_str().to_owned(),
            last_login_at: user.last_login_at.map(|value| value.unix_timestamp()),
            created_at: user.created_at.unix_timestamp(),
            updated_at: user.updated_at.unix_timestamp(),
        }
    }
}
