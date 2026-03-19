mod google;
mod jwt;

pub use google::{GoogleAuthService, GoogleAuthServiceBuilder, GoogleUserProfile};
pub use jwt::{
    ACCESS_TOKEN_EXPIRES_IN_SECS, AuthService, AuthServiceBuilder, REFRESH_TOKEN_EXPIRES_IN_SECS,
};

use jsonwebtoken::errors::Error as JwtError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("jwt error: {0}")]
    Jwt(#[from] JwtError),
    #[error("jwt config field `{field}` cannot be empty")]
    EmptySecret { field: &'static str },
    #[error("google client_id cannot be empty")]
    EmptyGoogleClientId,
    #[error("google client_secret cannot be empty")]
    EmptyGoogleClientSecret,
    #[error("google redirect_uri cannot be empty")]
    EmptyGoogleRedirectUri,
    #[allow(dead_code)]
    #[error("invalid token type: expected `{expected}`, got `{actual}`")]
    InvalidTokenType {
        expected: &'static str,
        actual: String,
    },
    #[error("invalid google authorization code")]
    InvalidGoogleAuthorizationCode,
    #[error("invalid google access token")]
    InvalidGoogleAccessToken,
    #[error("invalid google user info: {reason}")]
    InvalidGoogleUserInfo { reason: &'static str },
    #[error("google upstream request failed: {0}")]
    GoogleUpstream(#[from] reqwest::Error),
    #[error("google upstream responded with unexpected status {status}")]
    GoogleUpstreamStatus { status: u16 },
}
