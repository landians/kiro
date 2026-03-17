use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::config::AuthConfig;

#[derive(Clone)]
pub struct JwtService {
    issuer: String,
    audience: String,
    access_token_ttl_seconds: u64,
    refresh_token_ttl_seconds: u64,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtService {
    #[allow(dead_code)]
    pub fn issue_token_pair(&self, subject: &str, user_agent: &str) -> Result<TokenPair, JwtError> {
        Ok(TokenPair {
            access_token: self.issue_token(subject, user_agent, TokenKind::Access)?,
            refresh_token: self.issue_token(subject, user_agent, TokenKind::Refresh)?,
        })
    }

    pub fn issue_access_token(
        &self,
        subject: &str,
        user_agent: &str,
    ) -> Result<IssuedToken, JwtError> {
        self.issue_token(subject, user_agent, TokenKind::Access)
    }

    pub fn issue_refresh_token(
        &self,
        subject: &str,
        user_agent: &str,
    ) -> Result<IssuedToken, JwtError> {
        self.issue_token(subject, user_agent, TokenKind::Refresh)
    }

    pub fn validate_token(
        &self,
        token: &str,
        expected_kind: TokenKind,
    ) -> Result<ValidatedToken, JwtError> {
        let token_data =
            decode::<JwtClaims>(token, &self.decoding_key, &self.validation).map_err(|error| {
                match error.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::TokenExpired,
                    _ => JwtError::InvalidToken,
                }
            })?;

        if token_data.claims.token_kind != expected_kind.as_str() {
            return Err(JwtError::UnexpectedTokenKind {
                expected: expected_kind,
                actual: token_data.claims.token_kind.clone(),
            });
        }

        Ok(ValidatedToken {
            subject: token_data.claims.sub,
            jti: token_data.claims.jti,
            token_kind: expected_kind,
            ua_hash: token_data.claims.ua_hash,
            issued_at: token_data.claims.iat,
            expires_at: token_data.claims.exp,
        })
    }

    pub fn hash_user_agent(&self, user_agent: &str) -> Result<String, JwtError> {
        hash_user_agent(user_agent)
    }

    fn issue_token(
        &self,
        subject: &str,
        user_agent: &str,
        token_kind: TokenKind,
    ) -> Result<IssuedToken, JwtError> {
        if subject.trim().is_empty() {
            return Err(JwtError::MissingSubject);
        }

        let ua_hash = self.hash_user_agent(user_agent)?;
        let issued_at = current_unix_timestamp()?;
        let expires_at = issued_at + self.ttl_seconds(token_kind);
        let jti = Uuid::new_v4().to_string();

        let claims = JwtClaims {
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            sub: subject.to_owned(),
            jti: jti.clone(),
            ua_hash: ua_hash.clone(),
            token_kind: token_kind.as_str().to_owned(),
            iat: issued_at,
            exp: expires_at,
        };

        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|_| JwtError::TokenEncodingFailed)?;

        Ok(IssuedToken {
            token,
            subject: subject.to_owned(),
            jti,
            token_kind,
            ua_hash,
            issued_at,
            expires_at,
        })
    }

    fn ttl_seconds(&self, token_kind: TokenKind) -> u64 {
        match token_kind {
            TokenKind::Access => self.access_token_ttl_seconds,
            TokenKind::Refresh => self.refresh_token_ttl_seconds,
        }
    }
}

pub struct JwtServiceBuilder {
    config: AuthConfig,
}

impl JwtServiceBuilder {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    pub fn build(self) -> Result<JwtService> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.config.jwt_issuer.as_str()]);
        validation.set_audience(&[self.config.jwt_audience.as_str()]);

        Ok(JwtService {
            issuer: self.config.jwt_issuer.clone(),
            audience: self.config.jwt_audience.clone(),
            access_token_ttl_seconds: self.config.jwt_access_token_ttl_seconds,
            refresh_token_ttl_seconds: self.config.jwt_refresh_token_ttl_seconds,
            encoding_key: EncodingKey::from_secret(self.config.jwt_signing_key.as_bytes()),
            decoding_key: DecodingKey::from_secret(self.config.jwt_signing_key.as_bytes()),
            validation,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Access,
    Refresh,
}

impl TokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Refresh => "refresh",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: IssuedToken,
    pub refresh_token: IssuedToken,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IssuedToken {
    pub token: String,
    pub subject: String,
    pub jti: String,
    pub token_kind: TokenKind,
    pub ua_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone)]
pub struct ValidatedToken {
    pub subject: String,
    pub jti: String,
    pub token_kind: TokenKind,
    pub ua_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct JwtClaims {
    iss: String,
    aud: String,
    sub: String,
    jti: String,
    ua_hash: String,
    token_kind: String,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("token subject is required")]
    MissingSubject,
    #[error("user-agent is required")]
    MissingUserAgent,
    #[error("failed to encode token")]
    TokenEncodingFailed,
    #[error("invalid token")]
    InvalidToken,
    #[error("token is expired")]
    TokenExpired,
    #[error("unexpected token kind `{actual}`, expected `{expected}`")]
    UnexpectedTokenKind { expected: TokenKind, actual: String },
    #[error("token user-agent hash does not match request")]
    UserAgentMismatch,
    #[error("token has been revoked")]
    TokenRevoked,
    #[error("failed to read current time")]
    ClockUnavailable,
}

pub fn hash_user_agent(user_agent: &str) -> Result<String, JwtError> {
    let trimmed = user_agent.trim();
    if trimmed.is_empty() {
        return Err(JwtError::MissingUserAgent);
    }

    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(trimmed.as_bytes());
    Ok(hex::encode(digest))
}

fn current_unix_timestamp() -> Result<u64, JwtError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| JwtError::ClockUnavailable)
}

#[cfg(test)]
mod tests {
    use crate::config::{AuthConfig, BlacklistMode, GoogleAuthConfig};

    use super::{JwtServiceBuilder, TokenKind};

    fn auth_config() -> AuthConfig {
        AuthConfig {
            jwt_issuer: "kiro".to_owned(),
            jwt_audience: "kiro-api".to_owned(),
            jwt_signing_key: "test_signing_key_that_is_long_enough_123".to_owned(),
            jwt_access_token_ttl_seconds: 7200,
            jwt_refresh_token_ttl_seconds: 1296000,
            blacklist_mode: BlacklistMode::Memory,
            google: GoogleAuthConfig {
                enabled: false,
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_owned(),
                token_url: "https://oauth2.googleapis.com/token".to_owned(),
                user_info_url: "https://openidconnect.googleapis.com/v1/userinfo".to_owned(),
                http_timeout_seconds: 10,
                oauth_state_ttl_seconds: 600,
            },
        }
    }

    #[test]
    fn issues_and_validates_access_token() {
        let service = JwtServiceBuilder::new(auth_config())
            .build()
            .expect("jwt service should build");
        let token = service
            .issue_access_token("user_1", "unit-test-agent")
            .expect("access token should issue");

        let validated = service
            .validate_token(&token.token, TokenKind::Access)
            .expect("access token should validate");

        assert_eq!(validated.subject, "user_1");
        assert_eq!(validated.token_kind, TokenKind::Access);
        assert_eq!(validated.ua_hash, token.ua_hash);
    }

    #[test]
    fn rejects_token_with_wrong_kind() {
        let service = JwtServiceBuilder::new(auth_config())
            .build()
            .expect("jwt service should build");
        let token = service
            .issue_refresh_token("user_1", "unit-test-agent")
            .expect("refresh token should issue");

        let error = service
            .validate_token(&token.token, TokenKind::Access)
            .expect_err("refresh token should not validate as access token");

        assert!(error.to_string().contains("unexpected token kind"));
    }
}
