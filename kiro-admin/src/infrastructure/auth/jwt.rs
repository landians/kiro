use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AuthError;
use crate::infrastructure::config::JwtConfig;

pub const ACCESS_TOKEN_EXPIRES_IN_SECS: i64 = 7 * 24 * 60 * 60;

const ACCESS_TOKEN_TTL_HOURS: i64 = 7 * 24;
const ACCESS_TOKEN_KIND: &str = "access";

pub struct AuthServiceBuilder {
    pub issuer: String,
    pub access_secret: String,
}

#[derive(Clone)]
pub struct AuthService {
    issuer: String,
    access_secret: String,
}

impl AuthServiceBuilder {
    pub fn new(config: JwtConfig) -> Self {
        Self {
            issuer: config.issuer,
            access_secret: config.access_secret,
        }
    }

    pub fn build(self) -> Result<AuthService, AuthError> {
        if self.access_secret.trim().is_empty() {
            return Err(AuthError::EmptySecret {
                field: "access_secret",
            });
        }

        Ok(AuthService {
            issuer: self.issuer,
            access_secret: self.access_secret,
        })
    }
}

impl AuthService {
    #[tracing::instrument(skip(self), fields(token.kind = "access"))]
    pub fn generate_access_token(&self, subject: &str) -> Result<String, AuthError> {
        let now = Utc::now();
        let expires_at = now + Duration::hours(ACCESS_TOKEN_TTL_HOURS);
        let claims = TokenClaims {
            sub: subject.to_owned(),
            token_type: ACCESS_TOKEN_KIND.to_owned(),
            iss: self.issuer.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };
        let encoding_key = EncodingKey::from_secret(self.access_secret.as_bytes());

        encode(&Header::new(Algorithm::HS256), &claims, &encoding_key).map_err(AuthError::from)
    }

    #[tracing::instrument(skip(self, token), fields(token.kind = "access"))]
    pub fn validate_access_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        let decoding_key = DecodingKey::from_secret(self.access_secret.as_bytes());

        let claims = decode::<TokenClaims>(token, &decoding_key, &validation)
            .map(|token_data| token_data.claims)
            .map_err(AuthError::from)?;

        if claims.token_type != ACCESS_TOKEN_KIND {
            return Err(AuthError::InvalidTokenType {
                expected: ACCESS_TOKEN_KIND,
                actual: claims.token_type,
            });
        }

        Ok(claims)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenClaims {
    pub sub: String,
    pub token_type: String,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
    pub jti: String,
}

#[cfg(test)]
mod tests {
    use super::{ACCESS_TOKEN_EXPIRES_IN_SECS, AuthService, AuthServiceBuilder};
    use crate::infrastructure::config::JwtConfig;

    fn build_auth_service() -> AuthService {
        AuthServiceBuilder::new(JwtConfig {
            issuer: "kiro-admin".to_owned(),
            access_secret: "admin-access-secret-for-tests".to_owned(),
        })
        .build()
        .expect("auth service should be built")
    }

    #[test]
    fn generate_and_validate_access_token() {
        let auth_service = build_auth_service();

        let access_token = auth_service
            .generate_access_token("admin-123")
            .expect("access token should be generated");
        let claims = auth_service
            .validate_access_token(&access_token)
            .expect("access token should be valid");

        assert_eq!(claims.sub, "admin-123");
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.iss, "kiro-admin");
        assert!(claims.exp - claims.iat <= ACCESS_TOKEN_EXPIRES_IN_SECS);
    }
}
