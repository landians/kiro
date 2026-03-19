use reqwest::{Client, StatusCode};
use serde::Deserialize;
use urlencoding::encode;

use super::AuthError;
use crate::infrastructure::config::GoogleConfig;

const GOOGLE_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/userinfo?alt=json";
const GOOGLE_AUTHORIZE_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";

pub struct GoogleAuthServiceBuilder {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
}

#[derive(Clone)]
pub struct GoogleAuthService {
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    transport: GoogleAuthTransport,
}

#[derive(Clone)]
enum GoogleAuthTransport {
    Http {
        http_client: Client,
        token_endpoint: String,
        userinfo_endpoint: String,
    },
}

impl GoogleAuthServiceBuilder {
    pub fn new(config: GoogleConfig) -> Self {
        Self {
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            token_endpoint: GOOGLE_TOKEN_ENDPOINT.to_owned(),
            userinfo_endpoint: GOOGLE_USERINFO_ENDPOINT.to_owned(),
        }
    }

    pub fn build(self) -> Result<GoogleAuthService, AuthError> {
        if self.client_id.trim().is_empty() {
            return Err(AuthError::EmptyGoogleClientId);
        }

        if self.client_secret.trim().is_empty() {
            return Err(AuthError::EmptyGoogleClientSecret);
        }

        if self.redirect_uri.trim().is_empty() {
            return Err(AuthError::EmptyGoogleRedirectUri);
        }

        let transport = GoogleAuthTransport::Http {
            http_client: Client::new(),
            token_endpoint: self.token_endpoint,
            userinfo_endpoint: self.userinfo_endpoint,
        };

        Ok(GoogleAuthService {
            client_id: self.client_id,
            client_secret: self.client_secret,
            redirect_uri: self.redirect_uri,
            transport,
        })
    }
}

impl GoogleAuthService {
    pub fn build_authorization_url(&self, state: &str) -> String {
        let encoded_client_id = encode(&self.client_id);
        let encoded_redirect_uri = encode(&self.redirect_uri);
        let encoded_scope = encode("openid email profile");
        let encoded_state = encode(state);

        format!(
            "{GOOGLE_AUTHORIZE_ENDPOINT}?client_id={encoded_client_id}&redirect_uri={encoded_redirect_uri}&response_type=code&scope={encoded_scope}&access_type=online&include_granted_scopes=true&state={encoded_state}&prompt=select_account"
        )
    }

    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    pub async fn login_with_code(&self, code: &str) -> Result<GoogleUserProfile, AuthError> {
        let access_token = self.exchange_code_for_access_token(code).await?;
        self.fetch_user_info(&access_token).await
    }

    async fn exchange_code_for_access_token(&self, code: &str) -> Result<String, AuthError> {
        match &self.transport {
            GoogleAuthTransport::Http {
                http_client,
                token_endpoint,
                ..
            } => {
                let response = http_client
                    .post(token_endpoint)
                    .form(&[
                        ("code", code),
                        ("client_id", self.client_id.as_str()),
                        ("client_secret", self.client_secret.as_str()),
                        ("redirect_uri", self.redirect_uri.as_str()),
                        ("grant_type", "authorization_code"),
                    ])
                    .send()
                    .await
                    .map_err(AuthError::GoogleUpstream)?;

                match response.status() {
                    status if status.is_success() => {
                        let payload = response
                            .json::<GoogleTokenResponse>()
                            .await
                            .map_err(AuthError::GoogleUpstream)?;

                        Ok(payload.access_token)
                    }
                    StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED => {
                        Err(AuthError::InvalidGoogleAuthorizationCode)
                    }
                    status => Err(AuthError::GoogleUpstreamStatus {
                        status: status.as_u16(),
                    }),
                }
            }
        }
    }

    async fn fetch_user_info(&self, access_token: &str) -> Result<GoogleUserProfile, AuthError> {
        match &self.transport {
            GoogleAuthTransport::Http {
                http_client,
                userinfo_endpoint,
                ..
            } => {
                let response = http_client
                    .get(userinfo_endpoint)
                    .bearer_auth(access_token)
                    .send()
                    .await
                    .map_err(AuthError::GoogleUpstream)?;

                match response.status() {
                    status if status.is_success() => {
                        let payload = response
                            .json::<GoogleUserInfoResponse>()
                            .await
                            .map_err(AuthError::GoogleUpstream)?;

                        payload.try_into()
                    }
                    StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED => {
                        Err(AuthError::InvalidGoogleAccessToken)
                    }
                    status => Err(AuthError::GoogleUpstreamStatus {
                        status: status.as_u16(),
                    }),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GoogleUserInfoResponse {
    sub: String,
    email: String,
    email_verified: bool,
    name: Option<String>,
    given_name: Option<String>,
    family_name: Option<String>,
    picture: Option<String>,
}

impl TryFrom<GoogleUserInfoResponse> for GoogleUserProfile {
    type Error = AuthError;

    fn try_from(value: GoogleUserInfoResponse) -> Result<Self, Self::Error> {
        if !value.email_verified {
            return Err(AuthError::InvalidGoogleUserInfo {
                reason: "google email is not verified",
            });
        }

        Ok(Self {
            provider: "google".to_owned(),
            sub: value.sub,
            email: value.email,
            email_verified: true,
            name: value.name,
            given_name: value.given_name,
            family_name: value.family_name,
            picture: value.picture,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct GoogleUserProfile {
    pub provider: String,
    pub sub: String,
    pub email: String,
    pub email_verified: bool,
    pub name: Option<String>,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub picture: Option<String>,
}
