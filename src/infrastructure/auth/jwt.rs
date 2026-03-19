use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AuthError;
use crate::infrastructure::config::JwtConfig;

pub const ACCESS_TOKEN_EXPIRES_IN_SECS: i64 = 60 * 60;
pub const REFRESH_TOKEN_EXPIRES_IN_SECS: i64 = 15 * 24 * 60 * 60;

const ACCESS_TOKEN_TTL_HOURS: i64 = 1;
const REFRESH_TOKEN_TTL_HOURS: i64 = 15 * 24;
const ACCESS_TOKEN_KIND: &str = "access";
const REFRESH_TOKEN_KIND: &str = "refresh";

pub struct AuthServiceBuilder {
    pub issuer: String,
    pub access_secret: String,
    pub refresh_secret: String,
}

#[derive(Clone)]
pub struct AuthService {
    issuer: String,
    access_secret: String,
    refresh_secret: String,
}

impl AuthServiceBuilder {
    pub fn new(config: JwtConfig) -> Self {
        Self {
            issuer: config.issuer,
            access_secret: config.access_secret,
            refresh_secret: config.refresh_secret,
        }
    }

    pub fn build(self) -> Result<AuthService, AuthError> {
        if self.access_secret.trim().is_empty() {
            return Err(AuthError::EmptySecret {
                field: "access_secret",
            });
        }

        if self.refresh_secret.trim().is_empty() {
            return Err(AuthError::EmptySecret {
                field: "refresh_secret",
            });
        }

        Ok(AuthService {
            issuer: self.issuer,
            access_secret: self.access_secret,
            refresh_secret: self.refresh_secret,
        })
    }
}

impl AuthService {
    pub fn generate_access_token(&self, subject: &str) -> Result<String, AuthError> {
        self.generate_token(
            subject,
            ACCESS_TOKEN_KIND,
            Duration::hours(ACCESS_TOKEN_TTL_HOURS),
            &self.access_secret,
        )
    }

    #[allow(dead_code)]
    pub fn validate_access_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        self.validate_token(token, ACCESS_TOKEN_KIND, &self.access_secret)
    }

    pub fn generate_refresh_token(&self, subject: &str) -> Result<String, AuthError> {
        self.generate_token(
            subject,
            REFRESH_TOKEN_KIND,
            Duration::hours(REFRESH_TOKEN_TTL_HOURS),
            &self.refresh_secret,
        )
    }

    #[allow(dead_code)]
    pub fn validate_refresh_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        self.validate_token(token, REFRESH_TOKEN_KIND, &self.refresh_secret)
    }

    fn generate_token(
        &self,
        subject: &str,
        token_type: &'static str,
        ttl: Duration,
        secret: &str,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let expires_at = now + ttl;

        let claims = TokenClaims {
            sub: subject.to_owned(),
            token_type: token_type.to_owned(),
            iss: self.issuer.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: Uuid::new_v4().to_string(),
        };

        let encoding_key = EncodingKey::from_secret(secret.as_bytes());

        encode(&Header::new(Algorithm::HS256), &claims, &encoding_key).map_err(AuthError::from)
    }

    #[allow(dead_code)]
    fn validate_token(
        &self,
        token: &str,
        expected_type: &'static str,
        secret: &str,
    ) -> Result<TokenClaims, AuthError> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        let claims = decode::<TokenClaims>(token, &decoding_key, &validation)
            .map(|token_data| token_data.claims)
            .map_err(AuthError::from)?;

        if claims.token_type != expected_type {
            return Err(AuthError::InvalidTokenType {
                expected: expected_type,
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
    use super::{
        ACCESS_TOKEN_EXPIRES_IN_SECS, AuthService, AuthServiceBuilder,
        REFRESH_TOKEN_EXPIRES_IN_SECS,
    };
    use crate::infrastructure::config::JwtConfig;

    fn build_auth_service() -> AuthService {
        AuthServiceBuilder::new(JwtConfig {
            issuer: "kiro-api".to_owned(),
            access_secret: "access-secret-for-tests".to_owned(),
            refresh_secret: "refresh-secret-for-tests".to_owned(),
        })
        .build()
        .expect("auth service should be built")
    }

    #[test]
    fn generate_and_validate_access_token() {
        let auth_service = build_auth_service();

        let token = auth_service
            .generate_access_token("user-123")
            .expect("access token should be generated");
        let claims = auth_service
            .validate_access_token(&token)
            .expect("access token should be valid");

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.iss, "kiro-api");
        assert!(claims.exp - claims.iat <= ACCESS_TOKEN_EXPIRES_IN_SECS);
    }

    #[test]
    fn generate_and_validate_refresh_token() {
        let auth_service = build_auth_service();

        let token = auth_service
            .generate_refresh_token("user-123")
            .expect("refresh token should be generated");
        let claims = auth_service
            .validate_refresh_token(&token)
            .expect("refresh token should be valid");

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.token_type, "refresh");
        assert_eq!(claims.iss, "kiro-api");
        assert!(claims.exp - claims.iat <= REFRESH_TOKEN_EXPIRES_IN_SECS);
    }

    #[test]
    fn reject_access_validation_for_refresh_token() {
        let auth_service = build_auth_service();

        let token = auth_service
            .generate_refresh_token("user-123")
            .expect("refresh token should be generated");
        let err = auth_service
            .validate_access_token(&token)
            .expect_err("refresh token should not validate as access token");

        assert_eq!(err.to_string(), "jwt error: InvalidSignature".to_owned());
    }

    #[test]
    fn reject_empty_access_secret_when_building_auth_service() {
        let result = AuthServiceBuilder::new(JwtConfig {
            issuer: "kiro-api".to_owned(),
            access_secret: String::new(),
            refresh_secret: "refresh-secret-for-tests".to_owned(),
        })
        .build();

        let Err(err) = result else {
            panic!("builder should reject empty access secret");
        };

        assert_eq!(
            err.to_string(),
            "jwt config field `access_secret` cannot be empty"
        );
    }
}
