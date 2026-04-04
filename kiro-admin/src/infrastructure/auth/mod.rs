mod jwt;
pub mod password;

pub use jwt::{ACCESS_TOKEN_EXPIRES_IN_SECS, AuthService, AuthServiceBuilder, TokenClaims};

use jsonwebtoken::errors::Error as JwtError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("jwt error: {0}")]
    Jwt(#[from] JwtError),
    #[error("jwt config field `{field}` cannot be empty")]
    EmptySecret { field: &'static str },
    #[error("invalid token type: expected `{expected}`, got `{actual}`")]
    InvalidTokenType {
        expected: &'static str,
        actual: String,
    },
}
