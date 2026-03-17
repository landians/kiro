use std::time::Duration;

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::GoogleAuthConfig;

const GOOGLE_DEFAULT_SCOPE: &str = "openid email profile";

#[cfg(test)]
#[derive(Clone, Debug)]
enum GoogleOAuthTestError {
    GoogleApiRejected { status: u16, body: String },
    MissingAccessToken,
}

#[cfg(test)]
impl From<GoogleOAuthTestError> for GoogleOAuthError {
    fn from(value: GoogleOAuthTestError) -> Self {
        match value {
            GoogleOAuthTestError::GoogleApiRejected { status, body } => {
                GoogleOAuthError::GoogleApiRejected { status, body }
            }
            GoogleOAuthTestError::MissingAccessToken => GoogleOAuthError::MissingAccessToken,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
struct GoogleOAuthTestFixtures {
    token_response: GoogleTokenResponse,
    user_profile: GoogleUserProfile,
    token_error: Option<GoogleOAuthTestError>,
    profile_error: Option<GoogleOAuthTestError>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct GoogleOAuthClient {
    client: Client,
    authorization_url: String,
    token_url: String,
    user_info_url: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    #[cfg(test)]
    test_fixtures: Option<GoogleOAuthTestFixtures>,
}

#[allow(dead_code)]
impl GoogleOAuthClient {
    pub fn build_authorization_url(
        &self,
        state: &str,
        nonce: &str,
    ) -> Result<String, GoogleOAuthError> {
        if state.trim().is_empty() {
            return Err(GoogleOAuthError::MissingState);
        }

        if nonce.trim().is_empty() {
            return Err(GoogleOAuthError::MissingNonce);
        }

        let query = [
            ("response_type", "code".to_owned()),
            ("client_id", self.client_id.clone()),
            ("redirect_uri", self.redirect_uri.clone()),
            ("scope", GOOGLE_DEFAULT_SCOPE.to_owned()),
            ("state", state.trim().to_owned()),
            ("nonce", nonce.trim().to_owned()),
            ("access_type", "offline".to_owned()),
            ("include_granted_scopes", "true".to_owned()),
            ("prompt", "consent".to_owned()),
        ];

        let encoded_query = query
            .iter()
            .map(|(key, value)| format!("{key}={}", urlencoding::encode(value)))
            .collect::<Vec<_>>()
            .join("&");

        Ok(format!("{}?{encoded_query}", self.authorization_url))
    }

    pub async fn exchange_authorization_code(
        &self,
        authorization_code: &str,
    ) -> Result<GoogleTokenResponse, GoogleOAuthError> {
        if authorization_code.trim().is_empty() {
            return Err(GoogleOAuthError::MissingAuthorizationCode);
        }

        #[cfg(test)]
        if let Some(test_fixtures) = &self.test_fixtures {
            if let Some(error) = test_fixtures.token_error.clone() {
                return Err(error.into());
            }
            return Ok(test_fixtures.token_response.clone());
        }

        let response = self
            .client
            .post(&self.token_url)
            .form(&[
                ("code", authorization_code.trim()),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(GoogleOAuthError::TokenRequestFailed)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<unavailable>"));
            return Err(GoogleOAuthError::GoogleApiRejected { status, body });
        }

        response
            .json::<GoogleTokenResponse>()
            .await
            .map_err(GoogleOAuthError::TokenResponseInvalid)
    }

    pub async fn fetch_user_profile(
        &self,
        access_token: &str,
    ) -> Result<GoogleUserProfile, GoogleOAuthError> {
        if access_token.trim().is_empty() {
            return Err(GoogleOAuthError::MissingAccessToken);
        }

        #[cfg(test)]
        if let Some(test_fixtures) = &self.test_fixtures {
            if let Some(error) = test_fixtures.profile_error.clone() {
                return Err(error.into());
            }
            return Ok(test_fixtures.user_profile.clone());
        }

        let response = self
            .client
            .get(&self.user_info_url)
            .bearer_auth(access_token.trim())
            .send()
            .await
            .map_err(GoogleOAuthError::UserInfoRequestFailed)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("<unavailable>"));
            return Err(GoogleOAuthError::GoogleApiRejected { status, body });
        }

        response
            .json::<GoogleUserProfile>()
            .await
            .map_err(GoogleOAuthError::UserInfoResponseInvalid)
    }
}

pub struct GoogleOAuthClientBuilder {
    config: GoogleAuthConfig,
}

impl GoogleOAuthClientBuilder {
    pub fn new(config: GoogleAuthConfig) -> Self {
        Self { config }
    }

    pub fn build(self) -> Result<Option<GoogleOAuthClient>, GoogleOAuthError> {
        if !self.config.enabled {
            return Ok(None);
        }

        let client_id = required_config_value(self.config.client_id, "client_id")?;
        let client_secret = required_config_value(self.config.client_secret, "client_secret")?;
        let redirect_uri = required_config_value(self.config.redirect_uri, "redirect_uri")?;

        let client = Client::builder()
            .timeout(Duration::from_secs(self.config.http_timeout_seconds))
            .build()
            .map_err(GoogleOAuthError::HttpClientBuildFailed)?;

        Ok(Some(GoogleOAuthClient {
            client,
            authorization_url: self.config.authorization_url,
            token_url: self.config.token_url,
            user_info_url: self.config.user_info_url,
            client_id,
            client_secret,
            redirect_uri,
            #[cfg(test)]
            test_fixtures: None,
        }))
    }
}

#[cfg(test)]
impl GoogleOAuthClient {
    pub fn for_test(
        config: GoogleAuthConfig,
        token_response: GoogleTokenResponse,
        user_profile: GoogleUserProfile,
    ) -> Self {
        Self {
            client: Client::builder()
                .build()
                .expect("test google oauth http client should build"),
            authorization_url: config.authorization_url,
            token_url: config.token_url,
            user_info_url: config.user_info_url,
            client_id: config
                .client_id
                .expect("test google client id should be configured"),
            client_secret: config
                .client_secret
                .expect("test google client secret should be configured"),
            redirect_uri: config
                .redirect_uri
                .expect("test google redirect uri should be configured"),
            test_fixtures: Some(GoogleOAuthTestFixtures {
                token_response,
                user_profile,
                token_error: None,
                profile_error: None,
            }),
        }
    }

    pub fn for_test_exchange_error(config: GoogleAuthConfig, status: u16, body: &str) -> Self {
        Self {
            client: Client::builder()
                .build()
                .expect("test google oauth http client should build"),
            authorization_url: config.authorization_url,
            token_url: config.token_url,
            user_info_url: config.user_info_url,
            client_id: config
                .client_id
                .expect("test google client id should be configured"),
            client_secret: config
                .client_secret
                .expect("test google client secret should be configured"),
            redirect_uri: config
                .redirect_uri
                .expect("test google redirect uri should be configured"),
            test_fixtures: Some(GoogleOAuthTestFixtures {
                token_response: GoogleTokenResponse {
                    access_token: "unused-access-token".to_owned(),
                    expires_in: Some(3600),
                    refresh_token: None,
                    scope: None,
                    id_token: None,
                    token_type: Some("Bearer".to_owned()),
                },
                user_profile: GoogleUserProfile {
                    sub: "unused-subject".to_owned(),
                    email: None,
                    email_verified: None,
                    name: None,
                    given_name: None,
                    family_name: None,
                    picture: None,
                    locale: None,
                },
                token_error: Some(GoogleOAuthTestError::GoogleApiRejected {
                    status,
                    body: body.to_owned(),
                }),
                profile_error: None,
            }),
        }
    }

    pub fn for_test_profile_error(
        config: GoogleAuthConfig,
        token_response: GoogleTokenResponse,
    ) -> Self {
        Self {
            client: Client::builder()
                .build()
                .expect("test google oauth http client should build"),
            authorization_url: config.authorization_url,
            token_url: config.token_url,
            user_info_url: config.user_info_url,
            client_id: config
                .client_id
                .expect("test google client id should be configured"),
            client_secret: config
                .client_secret
                .expect("test google client secret should be configured"),
            redirect_uri: config
                .redirect_uri
                .expect("test google redirect uri should be configured"),
            test_fixtures: Some(GoogleOAuthTestFixtures {
                token_response,
                user_profile: GoogleUserProfile {
                    sub: "unused-subject".to_owned(),
                    email: None,
                    email_verified: None,
                    name: None,
                    given_name: None,
                    family_name: None,
                    picture: None,
                    locale: None,
                },
                token_error: None,
                profile_error: Some(GoogleOAuthTestError::MissingAccessToken),
            }),
        }
    }
}

fn required_config_value(
    value: Option<String>,
    field: &'static str,
) -> Result<String, GoogleOAuthError> {
    match value.map(|value| value.trim().to_owned()) {
        Some(value) if !value.is_empty() => Ok(value),
        _ => Err(GoogleOAuthError::MissingConfiguration { field }),
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub id_token: Option<String>,
    #[serde(default)]
    pub token_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleUserProfile {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub given_name: Option<String>,
    #[serde(default)]
    pub family_name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum GoogleOAuthError {
    #[error("google oauth configuration field `{field}` is required")]
    MissingConfiguration { field: &'static str },
    #[error("oauth state is required")]
    MissingState,
    #[error("oauth nonce is required")]
    MissingNonce,
    #[error("authorization code is required")]
    MissingAuthorizationCode,
    #[error("access token is required")]
    MissingAccessToken,
    #[error("failed to build google oauth http client")]
    HttpClientBuildFailed(reqwest::Error),
    #[error("google token request failed")]
    TokenRequestFailed(reqwest::Error),
    #[error("google token response payload is invalid")]
    TokenResponseInvalid(reqwest::Error),
    #[error("google user info request failed")]
    UserInfoRequestFailed(reqwest::Error),
    #[error("google user info response payload is invalid")]
    UserInfoResponseInvalid(reqwest::Error),
    #[error("google api rejected request with status {status}: {body}")]
    GoogleApiRejected { status: u16, body: String },
}

#[cfg(test)]
mod tests {
    use super::{GoogleOAuthClientBuilder, GoogleOAuthError};
    use crate::config::GoogleAuthConfig;

    fn enabled_google_config() -> GoogleAuthConfig {
        GoogleAuthConfig {
            enabled: true,
            client_id: Some("google-client-id".to_owned()),
            client_secret: Some("google-client-secret".to_owned()),
            redirect_uri: Some("http://localhost:3000/auth/google/callback".to_owned()),
            authorization_url: "https://accounts.google.com/o/oauth2/v2/auth".to_owned(),
            token_url: "https://oauth2.googleapis.com/token".to_owned(),
            user_info_url: "https://openidconnect.googleapis.com/v1/userinfo".to_owned(),
            http_timeout_seconds: 10,
            oauth_state_ttl_seconds: 600,
        }
    }

    #[test]
    fn builder_returns_none_when_google_auth_is_disabled() {
        let mut config = enabled_google_config();
        config.enabled = false;

        let client = GoogleOAuthClientBuilder::new(config)
            .build()
            .expect("builder should succeed");

        assert!(client.is_none());
    }

    #[test]
    fn builder_rejects_missing_required_config() {
        let mut config = enabled_google_config();
        config.client_secret = None;

        let error = GoogleOAuthClientBuilder::new(config)
            .build()
            .expect_err("builder should fail");

        assert!(matches!(
            error,
            GoogleOAuthError::MissingConfiguration {
                field: "client_secret"
            }
        ));
    }

    #[test]
    fn authorization_url_contains_required_query_parameters() {
        let client = GoogleOAuthClientBuilder::new(enabled_google_config())
            .build()
            .expect("builder should succeed")
            .expect("client should be enabled");

        let url = client
            .build_authorization_url("state-123", "nonce-456")
            .expect("authorization url should build");

        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("client_id=google-client-id"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("nonce=nonce-456"));
        assert!(url.contains("scope=openid%20email%20profile"));
    }
}
