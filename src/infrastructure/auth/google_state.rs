use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::config::AuthConfig;

const GOOGLE_OAUTH_STATE_PURPOSE: &str = "google_oauth_state";

#[derive(Clone)]
pub struct GoogleOAuthStateService {
    issuer: String,
    audience: String,
    ttl_seconds: u64,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl GoogleOAuthStateService {
    pub fn issue_state(&self, nonce: &str) -> Result<String, GoogleOAuthStateError> {
        if nonce.trim().is_empty() {
            return Err(GoogleOAuthStateError::MissingNonce);
        }

        let issued_at = current_unix_timestamp()?;
        let claims = GoogleOAuthStateClaims {
            iss: self.issuer.clone(),
            aud: self.audience.clone(),
            jti: Uuid::new_v4().to_string(),
            purpose: GOOGLE_OAUTH_STATE_PURPOSE.to_owned(),
            nonce: nonce.trim().to_owned(),
            iat: issued_at,
            exp: issued_at + self.ttl_seconds,
        };

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key)
            .map_err(|_| GoogleOAuthStateError::StateEncodingFailed)
    }

    pub fn validate_state(
        &self,
        state: &str,
    ) -> Result<ValidatedGoogleOAuthState, GoogleOAuthStateError> {
        if state.trim().is_empty() {
            return Err(GoogleOAuthStateError::MissingState);
        }

        let token_data =
            decode::<GoogleOAuthStateClaims>(state, &self.decoding_key, &self.validation).map_err(
                |error| match error.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                        GoogleOAuthStateError::StateExpired
                    }
                    _ => GoogleOAuthStateError::InvalidState,
                },
            )?;

        if token_data.claims.purpose != GOOGLE_OAUTH_STATE_PURPOSE {
            return Err(GoogleOAuthStateError::InvalidState);
        }

        if token_data.claims.nonce.trim().is_empty() {
            return Err(GoogleOAuthStateError::InvalidState);
        }

        Ok(ValidatedGoogleOAuthState {
            nonce: token_data.claims.nonce,
            issued_at: token_data.claims.iat,
            expires_at: token_data.claims.exp,
        })
    }
}

pub struct GoogleOAuthStateServiceBuilder {
    config: AuthConfig,
}

impl GoogleOAuthStateServiceBuilder {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    pub fn build(self) -> Result<Option<GoogleOAuthStateService>> {
        if !self.config.google.enabled {
            return Ok(None);
        }

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[self.config.jwt_issuer.as_str()]);
        validation.set_audience(&[self.config.jwt_audience.as_str()]);

        Ok(Some(GoogleOAuthStateService {
            issuer: self.config.jwt_issuer.clone(),
            audience: self.config.jwt_audience.clone(),
            ttl_seconds: self.config.google.oauth_state_ttl_seconds,
            encoding_key: EncodingKey::from_secret(self.config.jwt_signing_key.as_bytes()),
            decoding_key: DecodingKey::from_secret(self.config.jwt_signing_key.as_bytes()),
            validation,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct ValidatedGoogleOAuthState {
    pub nonce: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct GoogleOAuthStateClaims {
    iss: String,
    aud: String,
    jti: String,
    purpose: String,
    nonce: String,
    iat: u64,
    exp: u64,
}

#[derive(Debug, Error)]
pub enum GoogleOAuthStateError {
    #[error("google oauth nonce is required")]
    MissingNonce,
    #[error("google oauth state is required")]
    MissingState,
    #[error("failed to encode google oauth state")]
    StateEncodingFailed,
    #[error("google oauth state is invalid")]
    InvalidState,
    #[error("google oauth state is expired")]
    StateExpired,
    #[error("failed to read current time")]
    ClockUnavailable,
}

fn current_unix_timestamp() -> Result<u64, GoogleOAuthStateError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_| GoogleOAuthStateError::ClockUnavailable)
}

#[cfg(test)]
mod tests {
    use crate::config::{AuthConfig, BlacklistMode, GoogleAuthConfig};

    use super::{GoogleOAuthStateError, GoogleOAuthStateServiceBuilder};

    fn auth_config(google_enabled: bool) -> AuthConfig {
        AuthConfig {
            jwt_issuer: "kiro".to_owned(),
            jwt_audience: "kiro-api".to_owned(),
            jwt_signing_key: "test_signing_key_that_is_long_enough_123".to_owned(),
            jwt_access_token_ttl_seconds: 7200,
            jwt_refresh_token_ttl_seconds: 1296000,
            blacklist_mode: BlacklistMode::Memory,
            google: GoogleAuthConfig {
                enabled: google_enabled,
                client_id: Some("google-client-id".to_owned()),
                client_secret: Some("google-client-secret".to_owned()),
                redirect_uri: Some("http://localhost:3000/auth/google/callback".to_owned()),
                authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_owned(),
                token_url: "https://oauth2.googleapis.com/token".to_owned(),
                user_info_url: "https://openidconnect.googleapis.com/v1/userinfo".to_owned(),
                http_timeout_seconds: 10,
                oauth_state_ttl_seconds: 600,
            },
        }
    }

    #[test]
    fn builder_returns_none_when_google_auth_is_disabled() {
        let service = GoogleOAuthStateServiceBuilder::new(auth_config(false))
            .build()
            .expect("builder should succeed");

        assert!(service.is_none());
    }

    #[test]
    fn issued_state_can_be_validated() {
        let service = GoogleOAuthStateServiceBuilder::new(auth_config(true))
            .build()
            .expect("builder should succeed")
            .expect("state service should exist");

        let state = service
            .issue_state("nonce-123")
            .expect("state should issue");
        let validated = service
            .validate_state(&state)
            .expect("state should validate");

        assert_eq!(validated.nonce, "nonce-123");
        assert!(validated.expires_at >= validated.issued_at);
    }

    #[test]
    fn validate_state_rejects_invalid_token() {
        let service = GoogleOAuthStateServiceBuilder::new(auth_config(true))
            .build()
            .expect("builder should succeed")
            .expect("state service should exist");

        let error = service
            .validate_state("invalid-state")
            .expect_err("invalid state should fail");

        assert!(matches!(error, GoogleOAuthStateError::InvalidState));
    }
}
