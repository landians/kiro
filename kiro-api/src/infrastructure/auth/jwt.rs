use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use redis::{AsyncCommands, aio::MultiplexedConnection};
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
const REVOKED_TOKEN_KEY_PREFIX: &str = "auth:revoked";

pub struct AuthServiceBuilder {
    pub issuer: String,
    pub access_secret: String,
    pub refresh_secret: String,
    revoked_token_store: Option<MultiplexedConnection>,
}

#[derive(Clone)]
pub struct AuthService {
    issuer: String,
    access_secret: String,
    refresh_secret: String,
    revoked_token_store: Option<MultiplexedConnection>,
}

impl AuthServiceBuilder {
    pub fn new(config: JwtConfig) -> Self {
        Self {
            issuer: config.issuer,
            access_secret: config.access_secret,
            refresh_secret: config.refresh_secret,
            revoked_token_store: None,
        }
    }

    pub fn with_revoked_token_store(mut self, revoked_token_store: MultiplexedConnection) -> Self {
        self.revoked_token_store = Some(revoked_token_store);
        self
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
            revoked_token_store: self.revoked_token_store,
        })
    }
}

impl AuthService {
    #[tracing::instrument(skip(self), fields(token.kind = "pair"))]
    pub fn generate_token_pair(&self, subject: &str) -> Result<TokenPair, AuthError> {
        let jti = Uuid::new_v4().to_string();
        let access_token = self.generate_token(
            subject,
            ACCESS_TOKEN_KIND,
            Duration::hours(ACCESS_TOKEN_TTL_HOURS),
            &self.access_secret,
            &jti,
        )?;
        let refresh_token = self.generate_token(
            subject,
            REFRESH_TOKEN_KIND,
            Duration::hours(REFRESH_TOKEN_TTL_HOURS),
            &self.refresh_secret,
            &jti,
        )?;

        Ok(TokenPair {
            access_token,
            refresh_token,
        })
    }

    #[tracing::instrument(skip(self, token), fields(token.kind = "access"))]
    pub fn validate_access_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        self.validate_token(token, ACCESS_TOKEN_KIND, &self.access_secret)
    }

    #[tracing::instrument(skip(self, token), fields(token.kind = "access"))]
    pub async fn validate_active_access_token(
        &self,
        token: &str,
    ) -> Result<TokenClaims, AuthError> {
        let claims = self.validate_access_token(token)?;

        if self.is_token_revoked(&claims).await? {
            return Err(AuthError::RevokedToken);
        }

        Ok(claims)
    }

    #[allow(dead_code)]
    #[tracing::instrument(skip(self, token), fields(token.kind = "refresh"))]
    pub fn validate_refresh_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        self.validate_token(token, REFRESH_TOKEN_KIND, &self.refresh_secret)
    }

    #[tracing::instrument(skip(self, token), fields(token.kind = "refresh"))]
    pub async fn validate_active_refresh_token(
        &self,
        token: &str,
    ) -> Result<TokenClaims, AuthError> {
        let claims = self.validate_refresh_token(token)?;

        if self.is_token_revoked(&claims).await? {
            return Err(AuthError::RevokedToken);
        }

        Ok(claims)
    }

    #[tracing::instrument(skip(self, refresh_token))]
    pub async fn refresh_access_token(&self, refresh_token: &str) -> Result<String, AuthError> {
        let refresh_claims = self.validate_active_refresh_token(refresh_token).await?;

        self.generate_token(
            &refresh_claims.sub,
            ACCESS_TOKEN_KIND,
            Duration::hours(ACCESS_TOKEN_TTL_HOURS),
            &self.access_secret,
            &refresh_claims.jti,
        )
    }

    #[allow(dead_code)]
    #[tracing::instrument(skip(self, token))]
    pub async fn revoke_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        let claims = self.parse_token(token)?;
        self.revoke_claims(&claims).await?;

        Ok(claims)
    }

    fn generate_token(
        &self,
        subject: &str,
        token_type: &'static str,
        ttl: Duration,
        secret: &str,
        jti: &str,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let expires_at = now + ttl;

        let claims = TokenClaims {
            sub: subject.to_owned(),
            token_type: token_type.to_owned(),
            iss: self.issuer.clone(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
            jti: jti.to_owned(),
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

    fn parse_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        match self.validate_access_token(token) {
            Ok(claims) => Ok(claims),
            Err(access_error) => match self.validate_refresh_token(token) {
                Ok(claims) => Ok(claims),
                Err(_) => Err(access_error),
            },
        }
    }

    #[tracing::instrument(skip(self))]
    async fn is_token_revoked(&self, claims: &TokenClaims) -> Result<bool, AuthError> {
        let Some(mut revoked_token_store) = self.revoked_token_store.clone() else {
            return Err(AuthError::MissingRevokedTokenStore);
        };
        let key = build_revoked_token_key(claims);

        revoked_token_store
            .exists(key)
            .await
            .map_err(AuthError::from)
    }

    #[tracing::instrument(skip(self))]
    pub async fn revoke_claims(&self, claims: &TokenClaims) -> Result<(), AuthError> {
        let Some(ttl_seconds) = remaining_token_ttl_seconds(claims) else {
            return Ok(());
        };

        let Some(mut revoked_token_store) = self.revoked_token_store.clone() else {
            return Err(AuthError::MissingRevokedTokenStore);
        };
        let key = build_revoked_token_key(claims);

        revoked_token_store
            .set_ex::<_, _, ()>(key, "1", ttl_seconds)
            .await
            .map_err(AuthError::from)?;

        Ok(())
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

fn build_revoked_token_key(claims: &TokenClaims) -> String {
    format!("{REVOKED_TOKEN_KEY_PREFIX}:{}", claims.jti)
}

fn remaining_token_ttl_seconds(claims: &TokenClaims) -> Option<u64> {
    let remaining = claims.exp - Utc::now().timestamp();

    if remaining <= 0 {
        return None;
    }

    Some(remaining as u64)
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

        let token_pair = auth_service
            .generate_token_pair("user-123")
            .expect("token pair should be generated");
        let claims = auth_service
            .validate_access_token(&token_pair.access_token)
            .expect("access token should be valid");

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.token_type, "access");
        assert_eq!(claims.iss, "kiro-api");
        assert!(claims.exp - claims.iat <= ACCESS_TOKEN_EXPIRES_IN_SECS);
    }

    #[test]
    fn generate_and_validate_refresh_token() {
        let auth_service = build_auth_service();

        let token_pair = auth_service
            .generate_token_pair("user-123")
            .expect("token pair should be generated");
        let claims = auth_service
            .validate_refresh_token(&token_pair.refresh_token)
            .expect("refresh token should be valid");

        assert_eq!(claims.sub, "user-123");
        assert_eq!(claims.token_type, "refresh");
        assert_eq!(claims.iss, "kiro-api");
        assert!(claims.exp - claims.iat <= REFRESH_TOKEN_EXPIRES_IN_SECS);
    }

    #[test]
    fn generated_token_pair_shares_same_jti() {
        let auth_service = build_auth_service();

        let token_pair = auth_service
            .generate_token_pair("user-123")
            .expect("token pair should be generated");
        let access_claims = auth_service
            .validate_access_token(&token_pair.access_token)
            .expect("access token should be valid");
        let refresh_claims = auth_service
            .validate_refresh_token(&token_pair.refresh_token)
            .expect("refresh token should be valid");

        assert_eq!(access_claims.jti, refresh_claims.jti);
    }

    #[test]
    fn refresh_access_token_keeps_same_jti() {
        let auth_service = build_auth_service();

        let token_pair = auth_service
            .generate_token_pair("user-123")
            .expect("token pair should be generated");
        let refreshed_access_token = tokio::runtime::Runtime::new()
            .expect("runtime should be created")
            .block_on(auth_service.refresh_access_token(&token_pair.refresh_token))
            .expect("access token should be refreshed");
        let refreshed_access_claims = auth_service
            .validate_access_token(&refreshed_access_token)
            .expect("refreshed access token should be valid");
        let refresh_claims = auth_service
            .validate_refresh_token(&token_pair.refresh_token)
            .expect("refresh token should be valid");

        assert_eq!(refreshed_access_claims.jti, refresh_claims.jti);
        assert_eq!(refreshed_access_claims.sub, refresh_claims.sub);
        assert_eq!(refreshed_access_claims.token_type, "access");
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
