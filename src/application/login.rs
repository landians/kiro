use serde_json::to_value;
use thiserror::Error;
use uuid::Uuid;

use crate::application::account::AccountService;
use crate::application::auth::AuthService;
use crate::application::user_identity::UserIdentityService;
use crate::domain::account::{NewUser, UserStatus};
use crate::domain::repository::user::{UserRepository, UserRepositoryError};
use crate::domain::repository::user_identity::{
    UserIdentityRepository, UserIdentityRepositoryError,
};
use crate::domain::user_identity::{IdentityProvider, NewUserIdentity};
use crate::infrastructure::auth::google::{GoogleOAuthClient, GoogleOAuthError, GoogleUserProfile};
use crate::infrastructure::auth::google_state::{GoogleOAuthStateError, GoogleOAuthStateService};
use crate::infrastructure::auth::jwt::JwtError;
#[cfg(test)]
use crate::infrastructure::persistence::in_memory::accounts::user_identity_repository::InMemoryUserIdentityRepository;
#[cfg(test)]
use crate::infrastructure::persistence::in_memory::accounts::user_repository::InMemoryUserRepository;
use crate::infrastructure::persistence::postgres::accounts::user_identity_repository::PostgresUserIdentityRepository;
use crate::infrastructure::persistence::postgres::accounts::user_repository::PostgresUserRepository;

pub type DefaultLoginService = LoginService<PostgresUserRepository, PostgresUserIdentityRepository>;
#[cfg(test)]
pub type TestLoginService = LoginService<InMemoryUserRepository, InMemoryUserIdentityRepository>;

#[derive(Clone)]
pub struct LoginService<UR, IR>
where
    UR: UserRepository,
    IR: UserIdentityRepository,
{
    #[allow(dead_code)]
    account_service: Option<AccountService<UR>>,
    #[allow(dead_code)]
    auth_service: AuthService,
    google_oauth_client: Option<GoogleOAuthClient>,
    google_oauth_state_service: Option<GoogleOAuthStateService>,
    #[allow(dead_code)]
    user_identity_service: Option<UserIdentityService<IR>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleAuthorizationRequest {
    pub authorization_url: String,
    pub state: String,
    pub nonce: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleLoginCallbackCommand {
    pub authorization_code: String,
    pub oauth_state: String,
    pub user_agent: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleLoginResult {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: u64,
    pub refresh_token_expires_at: u64,
    pub identity_code: String,
    pub is_new_user: bool,
    pub provider: IdentityProvider,
    pub user_code: String,
}

impl<UR, IR> LoginService<UR, IR>
where
    UR: UserRepository,
    IR: UserIdentityRepository,
{
    pub fn new(
        account_service: Option<AccountService<UR>>,
        auth_service: AuthService,
        google_oauth_client: Option<GoogleOAuthClient>,
        google_oauth_state_service: Option<GoogleOAuthStateService>,
        user_identity_service: Option<UserIdentityService<IR>>,
    ) -> Self {
        Self {
            account_service,
            auth_service,
            google_oauth_client,
            google_oauth_state_service,
            user_identity_service,
        }
    }

    pub fn google_login_enabled(&self) -> bool {
        self.google_oauth_client.is_some() && self.google_oauth_state_service.is_some()
    }

    pub fn build_google_authorization_url(
        &self,
        state: &str,
        nonce: &str,
    ) -> Result<String, LoginServiceError> {
        let google_oauth_client = self
            .google_oauth_client
            .as_ref()
            .ok_or(LoginServiceError::GoogleLoginDisabled)?;

        google_oauth_client
            .build_authorization_url(state, nonce)
            .map_err(LoginServiceError::GoogleOAuth)
    }

    pub fn build_google_authorization_request(
        &self,
    ) -> Result<GoogleAuthorizationRequest, LoginServiceError> {
        let nonce = Uuid::new_v4().to_string();
        let google_oauth_state_service = self
            .google_oauth_state_service
            .as_ref()
            .ok_or(LoginServiceError::GoogleStateServiceUnavailable)?;
        let state = google_oauth_state_service
            .issue_state(&nonce)
            .map_err(LoginServiceError::GoogleOAuthState)?;
        let authorization_url = self.build_google_authorization_url(&state, &nonce)?;

        Ok(GoogleAuthorizationRequest {
            authorization_url,
            state,
            nonce,
        })
    }

    pub async fn complete_google_login(
        &self,
        command: GoogleLoginCallbackCommand,
    ) -> Result<GoogleLoginResult, LoginServiceError> {
        if command.authorization_code.trim().is_empty() {
            return Err(LoginServiceError::MissingAuthorizationCode);
        }

        if command.oauth_state.trim().is_empty() {
            return Err(LoginServiceError::MissingOAuthState);
        }

        if command.user_agent.trim().is_empty() {
            return Err(LoginServiceError::MissingUserAgent);
        }

        let google_oauth_client = self
            .google_oauth_client
            .as_ref()
            .ok_or(LoginServiceError::GoogleLoginDisabled)?;
        let google_oauth_state_service = self
            .google_oauth_state_service
            .as_ref()
            .ok_or(LoginServiceError::GoogleStateServiceUnavailable)?;
        let account_service = self
            .account_service
            .as_ref()
            .ok_or(LoginServiceError::AccountServiceUnavailable)?;
        let user_identity_service = self
            .user_identity_service
            .as_ref()
            .ok_or(LoginServiceError::UserIdentityServiceUnavailable)?;

        google_oauth_state_service
            .validate_state(&command.oauth_state)
            .map_err(LoginServiceError::GoogleOAuthState)?;

        let token_response = google_oauth_client
            .exchange_authorization_code(&command.authorization_code)
            .await
            .map_err(LoginServiceError::GoogleOAuth)?;
        let profile = google_oauth_client
            .fetch_user_profile(&token_response.access_token)
            .await
            .map_err(LoginServiceError::GoogleOAuth)?;

        let provider_subject = profile.sub.trim();
        if provider_subject.is_empty() {
            return Err(LoginServiceError::MissingGoogleSubject);
        }

        let authenticated_at = time::OffsetDateTime::now_utc();
        let profile_json =
            to_value(&profile).map_err(LoginServiceError::ProfileSerializationFailed)?;

        let (user, identity_code, is_new_user) = match user_identity_service
            .find_identity_by_provider_subject(IdentityProvider::Google, provider_subject)
            .await
            .map_err(LoginServiceError::UserIdentityRepository)?
        {
            Some(identity) => {
                user_identity_service
                    .record_authentication(identity.id)
                    .await
                    .map_err(LoginServiceError::UserIdentityRepository)?;

                let user = account_service
                    .find_user_by_id(identity.user_id)
                    .await
                    .map_err(LoginServiceError::UserRepository)?
                    .ok_or(LoginServiceError::InconsistentIdentityUser {
                        user_id: identity.user_id,
                    })?;

                (user, identity.identity_code, false)
            }
            None => {
                if let Some(provider_email) = verified_email(&profile) {
                    if let Some(existing_identity) = user_identity_service
                        .find_identity_by_provider_email(IdentityProvider::Google, &provider_email)
                        .await
                        .map_err(LoginServiceError::UserIdentityRepository)?
                    {
                        return Err(LoginServiceError::IdentityBindingConflict {
                            identity_code: existing_identity.identity_code,
                        });
                    }
                }

                let maybe_existing_user = match verified_email(&profile) {
                    Some(provider_email) => account_service
                        .find_user_by_email(&provider_email)
                        .await
                        .map_err(LoginServiceError::UserRepository)?,
                    None => None,
                };

                let (user, is_new_user) = match maybe_existing_user {
                    Some(user) => (user, false),
                    None => {
                        let new_user = new_user_from_google_profile(&profile);
                        let user = account_service
                            .create_user(new_user)
                            .await
                            .map_err(LoginServiceError::UserRepository)?;
                        (user, true)
                    }
                };

                let provider_email = verified_email(&profile).unwrap_or_default();
                let new_identity = NewUserIdentity::new(
                    generate_identity_code(),
                    user.id,
                    IdentityProvider::Google,
                    provider_subject.to_owned(),
                )
                .with_profile(profile_json)
                .with_last_authenticated_at(authenticated_at)
                .with_provider_email(provider_email);
                let identity = user_identity_service
                    .create_identity(new_identity)
                    .await
                    .map_err(LoginServiceError::UserIdentityRepository)?;

                (user, identity.identity_code, is_new_user)
            }
        };

        let user = account_service
            .record_last_login(user.id)
            .await
            .map_err(LoginServiceError::UserRepository)?;
        let token_pair = self
            .auth_service
            .issue_token_pair(&user.user_code, &command.user_agent)
            .map_err(LoginServiceError::TokenIssuanceFailed)?;

        Ok(GoogleLoginResult {
            access_token: token_pair.access_token.token,
            refresh_token: token_pair.refresh_token.token,
            access_token_expires_at: token_pair.access_token.expires_at,
            refresh_token_expires_at: token_pair.refresh_token.expires_at,
            identity_code,
            is_new_user,
            provider: IdentityProvider::Google,
            user_code: user.user_code,
        })
    }
}

#[derive(Debug, Error)]
pub enum LoginServiceError {
    #[error("google login is not enabled")]
    GoogleLoginDisabled,
    #[error("google callback authorization code is required")]
    MissingAuthorizationCode,
    #[error("google callback state is required")]
    MissingOAuthState,
    #[error("user agent is required")]
    MissingUserAgent,
    #[error("google oauth state service is not available")]
    GoogleStateServiceUnavailable,
    #[error("account service is not available")]
    AccountServiceUnavailable,
    #[error("user identity service is not available")]
    UserIdentityServiceUnavailable,
    #[error("google user profile does not contain a subject")]
    MissingGoogleSubject,
    #[error("user identity points to a missing user `{user_id}`")]
    InconsistentIdentityUser { user_id: i64 },
    #[error("google identity conflicts with existing identity `{identity_code}`")]
    IdentityBindingConflict { identity_code: String },
    #[error(transparent)]
    GoogleOAuth(#[from] GoogleOAuthError),
    #[error(transparent)]
    GoogleOAuthState(#[from] GoogleOAuthStateError),
    #[error(transparent)]
    UserRepository(#[from] UserRepositoryError),
    #[error(transparent)]
    UserIdentityRepository(#[from] UserIdentityRepositoryError),
    #[error("failed to serialize google profile")]
    ProfileSerializationFailed(serde_json::Error),
    #[error("failed to issue session tokens")]
    TokenIssuanceFailed(JwtError),
}

fn verified_email(profile: &GoogleUserProfile) -> Option<String> {
    if profile.email_verified != Some(true) {
        return None;
    }

    profile
        .email
        .as_deref()
        .map(str::trim)
        .filter(|email| !email.is_empty())
        .map(str::to_owned)
}

fn new_user_from_google_profile(profile: &GoogleUserProfile) -> NewUser {
    let mut new_user = NewUser::new(generate_user_code()).with_status(UserStatus::Active);

    if let Some(email) = verified_email(profile) {
        new_user = new_user.with_email(email);
    }

    if let Some(name) = profile.name.as_deref() {
        new_user = new_user.with_display_name(name);
    }

    if let Some(picture) = profile.picture.as_deref() {
        new_user = new_user.with_avatar_url(picture);
    }

    if let Some(locale) = profile.locale.as_deref() {
        new_user = new_user.with_locale(locale);
    }

    new_user
}

fn generate_user_code() -> String {
    format!("user_{}", Uuid::new_v4().simple())
}

fn generate_identity_code() -> String {
    format!("identity_{}", Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use time::OffsetDateTime;

    use crate::application::account::AccountService;
    use crate::application::auth::AuthService;
    use crate::application::login::{GoogleLoginCallbackCommand, LoginService, LoginServiceError};
    use crate::application::user_identity::UserIdentityService;
    use crate::config::{AuthConfig, BlacklistMode, GoogleAuthConfig};
    use crate::domain::account::{NewUser, User, UserStatus};
    use crate::domain::repository::user::{UserRepository, UserRepositoryError};
    use crate::domain::repository::user_identity::{
        UserIdentityRepository, UserIdentityRepositoryError,
    };
    use crate::domain::user_identity::{IdentityProvider, NewUserIdentity, UserIdentity};
    use crate::infrastructure::auth::blacklist::TokenBlacklistServiceBuilder;
    use crate::infrastructure::auth::google::{
        GoogleOAuthClient, GoogleTokenResponse, GoogleUserProfile,
    };
    use crate::infrastructure::auth::google_state::{
        GoogleOAuthStateError, GoogleOAuthStateService, GoogleOAuthStateServiceBuilder,
    };
    use crate::infrastructure::auth::jwt::JwtServiceBuilder;

    fn auth_service_for_test() -> AuthService {
        let auth_config = AuthConfig {
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
        };

        AuthService::new(
            JwtServiceBuilder::new(auth_config)
                .build()
                .expect("jwt service should build"),
            TokenBlacklistServiceBuilder::new(BlacklistMode::Memory).build(),
        )
    }

    fn google_client_for_test() -> GoogleOAuthClient {
        let config = google_auth_config_for_test();

        GoogleOAuthClient::for_test(
            config,
            GoogleTokenResponse {
                access_token: "google-access-token".to_owned(),
                expires_in: Some(3600),
                refresh_token: Some("google-refresh-token".to_owned()),
                scope: Some("openid email profile".to_owned()),
                id_token: Some("google-id-token".to_owned()),
                token_type: Some("Bearer".to_owned()),
            },
            GoogleUserProfile {
                sub: "google-subject-42".to_owned(),
                email: Some("hello@example.com".to_owned()),
                email_verified: Some(true),
                name: Some("Hello User".to_owned()),
                given_name: Some("Hello".to_owned()),
                family_name: Some("User".to_owned()),
                picture: Some("https://example.com/avatar.png".to_owned()),
                locale: Some("en-US".to_owned()),
            },
        )
    }

    fn google_client_for_exchange_error() -> GoogleOAuthClient {
        GoogleOAuthClient::for_test_exchange_error(
            google_auth_config_for_test(),
            502,
            "google exchange failed",
        )
    }

    fn google_client_for_profile_error() -> GoogleOAuthClient {
        GoogleOAuthClient::for_test_profile_error(
            google_auth_config_for_test(),
            GoogleTokenResponse {
                access_token: "google-access-token".to_owned(),
                expires_in: Some(3600),
                refresh_token: None,
                scope: Some("openid email profile".to_owned()),
                id_token: None,
                token_type: Some("Bearer".to_owned()),
            },
        )
    }

    fn google_auth_config_for_test() -> GoogleAuthConfig {
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

    fn google_state_service_for_test() -> GoogleOAuthStateService {
        GoogleOAuthStateServiceBuilder::new(AuthConfig {
            jwt_issuer: "kiro".to_owned(),
            jwt_audience: "kiro-api".to_owned(),
            jwt_signing_key: "test_signing_key_that_is_long_enough_123".to_owned(),
            jwt_access_token_ttl_seconds: 7200,
            jwt_refresh_token_ttl_seconds: 1296000,
            blacklist_mode: BlacklistMode::Memory,
            google: google_auth_config_for_test(),
        })
        .build()
        .expect("google oauth state builder should succeed")
        .expect("google oauth state service should exist")
    }

    #[derive(Clone, Default)]
    struct TestUserRepository {
        users: Arc<Mutex<HashMap<i64, User>>>,
    }

    impl TestUserRepository {
        fn seeded(user: User) -> Self {
            let mut users = HashMap::new();
            users.insert(user.id, user);
            let users = Arc::new(Mutex::new(users));
            Self { users }
        }
    }

    impl UserRepository for TestUserRepository {
        async fn create(&self, new_user: NewUser) -> Result<User, UserRepositoryError> {
            new_user.validate()?;

            let mut users = self.users.lock().expect("users mutex should lock");
            let user = User {
                id: (users.len() + 1) as i64,
                user_code: new_user.user_code,
                email: new_user.email,
                email_normalized: new_user.email_normalized,
                display_name: new_user.display_name,
                avatar_url: new_user.avatar_url,
                locale: new_user.locale,
                time_zone: new_user.time_zone,
                status: new_user.status,
                last_login_at: None,
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            };

            users.insert(user.id, user.clone());
            Ok(user)
        }

        async fn find_by_id(&self, user_id: i64) -> Result<Option<User>, UserRepositoryError> {
            Ok(self
                .users
                .lock()
                .expect("users mutex should lock")
                .get(&user_id)
                .cloned())
        }

        async fn find_by_user_code<'a>(
            &'a self,
            user_code: &'a str,
        ) -> Result<Option<User>, UserRepositoryError> {
            Ok(self
                .users
                .lock()
                .expect("users mutex should lock")
                .values()
                .find(|user| user.user_code == user_code)
                .cloned())
        }

        async fn find_by_email_normalized<'a>(
            &'a self,
            email_normalized: &'a str,
        ) -> Result<Option<User>, UserRepositoryError> {
            Ok(self
                .users
                .lock()
                .expect("users mutex should lock")
                .values()
                .find(|user| user.email_normalized.as_deref() == Some(email_normalized))
                .cloned())
        }

        async fn update_status(
            &self,
            user_id: i64,
            next_status: UserStatus,
        ) -> Result<User, UserRepositoryError> {
            let mut users = self.users.lock().expect("users mutex should lock");
            let user = users
                .get_mut(&user_id)
                .ok_or(UserRepositoryError::NotFound)?;
            user.transition_to(next_status)?;
            Ok(user.clone())
        }

        async fn update_last_login_at(
            &self,
            user_id: i64,
            last_login_at: OffsetDateTime,
        ) -> Result<User, UserRepositoryError> {
            let mut users = self.users.lock().expect("users mutex should lock");
            let user = users
                .get_mut(&user_id)
                .ok_or(UserRepositoryError::NotFound)?;
            user.last_login_at = Some(last_login_at);
            user.updated_at = last_login_at;
            Ok(user.clone())
        }
    }

    #[derive(Clone, Default)]
    struct TestUserIdentityRepository {
        identities: Arc<Mutex<HashMap<i64, UserIdentity>>>,
    }

    impl TestUserIdentityRepository {
        fn seeded(identity: UserIdentity) -> Self {
            let mut identities = HashMap::new();
            identities.insert(identity.id, identity);
            let identities = Arc::new(Mutex::new(identities));
            Self { identities }
        }
    }

    fn login_service_for_test(
        user_repository: TestUserRepository,
        user_identity_repository: TestUserIdentityRepository,
        google_oauth_client: GoogleOAuthClient,
    ) -> LoginService<TestUserRepository, TestUserIdentityRepository> {
        let account_service = AccountService::new(user_repository);
        let user_identity_service = UserIdentityService::new(user_identity_repository);
        let google_oauth_state_service = google_state_service_for_test();

        LoginService::new(
            Some(account_service),
            auth_service_for_test(),
            Some(google_oauth_client),
            Some(google_oauth_state_service),
            Some(user_identity_service),
        )
    }

    impl UserIdentityRepository for TestUserIdentityRepository {
        async fn create(
            &self,
            new_identity: NewUserIdentity,
        ) -> Result<UserIdentity, UserIdentityRepositoryError> {
            new_identity.validate()?;

            let mut identities = self
                .identities
                .lock()
                .expect("identities mutex should lock");

            if identities.values().any(|identity| {
                identity.provider == new_identity.provider
                    && identity.provider_user_id == new_identity.provider_user_id
            }) {
                return Err(UserIdentityRepositoryError::Conflict {
                    field: "provider_user_id",
                });
            }

            let identity = UserIdentity {
                id: (identities.len() + 1) as i64,
                identity_code: new_identity.identity_code,
                user_id: new_identity.user_id,
                provider: new_identity.provider,
                provider_user_id: new_identity.provider_user_id,
                provider_email: new_identity.provider_email,
                provider_email_normalized: new_identity.provider_email_normalized,
                profile: new_identity.profile,
                last_authenticated_at: new_identity.last_authenticated_at,
                created_at: OffsetDateTime::now_utc(),
                updated_at: OffsetDateTime::now_utc(),
            };

            identities.insert(identity.id, identity.clone());
            Ok(identity)
        }

        async fn find_by_id(
            &self,
            identity_id: i64,
        ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
            Ok(self
                .identities
                .lock()
                .expect("identities mutex should lock")
                .get(&identity_id)
                .cloned())
        }

        async fn find_by_identity_code<'a>(
            &'a self,
            identity_code: &'a str,
        ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
            Ok(self
                .identities
                .lock()
                .expect("identities mutex should lock")
                .values()
                .find(|identity| identity.identity_code == identity_code)
                .cloned())
        }

        async fn find_by_provider_subject<'a>(
            &'a self,
            provider: IdentityProvider,
            provider_user_id: &'a str,
        ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
            Ok(self
                .identities
                .lock()
                .expect("identities mutex should lock")
                .values()
                .find(|identity| {
                    identity.provider == provider && identity.provider_user_id == provider_user_id
                })
                .cloned())
        }

        async fn find_by_provider_email_normalized<'a>(
            &'a self,
            provider: IdentityProvider,
            provider_email_normalized: &'a str,
        ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
            Ok(self
                .identities
                .lock()
                .expect("identities mutex should lock")
                .values()
                .find(|identity| {
                    identity.provider == provider
                        && identity.provider_email_normalized.as_deref()
                            == Some(provider_email_normalized)
                })
                .cloned())
        }

        async fn list_by_user_id(
            &self,
            user_id: i64,
        ) -> Result<Vec<UserIdentity>, UserIdentityRepositoryError> {
            Ok(self
                .identities
                .lock()
                .expect("identities mutex should lock")
                .values()
                .filter(|identity| identity.user_id == user_id)
                .cloned()
                .collect())
        }

        async fn update_last_authenticated_at(
            &self,
            identity_id: i64,
            last_authenticated_at: OffsetDateTime,
        ) -> Result<UserIdentity, UserIdentityRepositoryError> {
            let mut identities = self
                .identities
                .lock()
                .expect("identities mutex should lock");
            let identity = identities
                .get_mut(&identity_id)
                .ok_or(UserIdentityRepositoryError::NotFound)?;
            identity.last_authenticated_at = Some(last_authenticated_at);
            identity.updated_at = last_authenticated_at;
            Ok(identity.clone())
        }
    }

    fn seeded_user() -> User {
        let now = OffsetDateTime::now_utc();
        User {
            id: 7,
            user_code: "user_existing".to_owned(),
            email: Some("hello@example.com".to_owned()),
            email_normalized: Some("hello@example.com".to_owned()),
            display_name: Some("Existing User".to_owned()),
            avatar_url: None,
            locale: "en-US".to_owned(),
            time_zone: "UTC".to_owned(),
            status: UserStatus::Active,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn seeded_identity() -> UserIdentity {
        let now = OffsetDateTime::now_utc();
        UserIdentity {
            id: 3,
            identity_code: "identity_existing".to_owned(),
            user_id: 7,
            provider: IdentityProvider::Google,
            provider_user_id: "google-subject-42".to_owned(),
            provider_email: Some("hello@example.com".to_owned()),
            provider_email_normalized: Some("hello@example.com".to_owned()),
            profile: json!({"sub": "google-subject-42"}),
            last_authenticated_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn seeded_conflicting_identity() -> UserIdentity {
        let now = OffsetDateTime::now_utc();
        UserIdentity {
            id: 8,
            identity_code: "identity_conflict".to_owned(),
            user_id: 99,
            provider: IdentityProvider::Google,
            provider_user_id: "other-google-subject".to_owned(),
            provider_email: Some("hello@example.com".to_owned()),
            provider_email_normalized: Some("hello@example.com".to_owned()),
            profile: json!({"sub": "other-google-subject"}),
            last_authenticated_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn build_google_authorization_url_requires_google_client() {
        let auth_service = auth_service_for_test();
        let login_service: LoginService<TestUserRepository, TestUserIdentityRepository> =
            LoginService::new(None, auth_service, None, None, None);

        let error = login_service
            .build_google_authorization_url("state-123", "nonce-456")
            .expect_err("google login should be disabled");

        assert!(matches!(error, LoginServiceError::GoogleLoginDisabled));
    }

    #[test]
    fn build_google_authorization_url_uses_google_adapter() {
        let auth_service = auth_service_for_test();
        let google_oauth_client = google_client_for_test();
        let google_oauth_state_service = google_state_service_for_test();
        let login_service: LoginService<TestUserRepository, TestUserIdentityRepository> =
            LoginService::new(
                None,
                auth_service,
                Some(google_oauth_client),
                Some(google_oauth_state_service),
                None,
            );

        let url = login_service
            .build_google_authorization_url("state-123", "nonce-456")
            .expect("authorization url should build");

        assert!(login_service.google_login_enabled());
        assert!(url.contains("client_id=google-client-id"));
        assert!(url.contains("state=state-123"));
        assert!(url.contains("nonce=nonce-456"));
    }

    #[test]
    fn build_google_authorization_request_generates_state_and_nonce() {
        let auth_service = auth_service_for_test();
        let google_oauth_client = google_client_for_test();
        let google_oauth_state_service = google_state_service_for_test();
        let login_service: LoginService<TestUserRepository, TestUserIdentityRepository> =
            LoginService::new(
                None,
                auth_service,
                Some(google_oauth_client),
                Some(google_oauth_state_service),
                None,
            );

        let request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        assert!(!request.state.is_empty());
        assert!(!request.nonce.is_empty());
        let state_query = format!("state={}", request.state);
        let nonce_query = format!("nonce={}", request.nonce);
        assert!(request.authorization_url.contains(&state_query));
        assert!(request.authorization_url.contains(&nonce_query));
    }

    #[tokio::test]
    async fn complete_google_login_creates_user_and_identity_for_first_login() {
        let login_service = login_service_for_test(
            TestUserRepository::default(),
            TestUserIdentityRepository::default(),
            google_client_for_test(),
        );
        let authorization_request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        let result = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: authorization_request.state,
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect("google login should succeed");

        assert!(result.is_new_user);
        assert!(result.user_code.starts_with("user_"));
        assert!(result.identity_code.starts_with("identity_"));
        assert!(!result.access_token.is_empty());
        assert!(!result.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn complete_google_login_binds_existing_user_by_verified_email() {
        let user_repository = TestUserRepository::seeded(seeded_user());
        let login_service = login_service_for_test(
            user_repository,
            TestUserIdentityRepository::default(),
            google_client_for_test(),
        );
        let authorization_request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        let result = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: authorization_request.state,
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect("google login should succeed");

        assert!(!result.is_new_user);
        assert_eq!(result.user_code, "user_existing");
    }

    #[tokio::test]
    async fn complete_google_login_reuses_existing_identity_binding() {
        let user_repository = TestUserRepository::seeded(seeded_user());
        let user_identity_repository = TestUserIdentityRepository::seeded(seeded_identity());
        let login_service = login_service_for_test(
            user_repository,
            user_identity_repository,
            google_client_for_test(),
        );
        let authorization_request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        let result = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: authorization_request.state,
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect("google login should succeed");

        assert_eq!(result.identity_code, "identity_existing");
        assert!(!result.is_new_user);
    }

    #[tokio::test]
    async fn complete_google_login_rejects_missing_oauth_state() {
        let login_service = login_service_for_test(
            TestUserRepository::default(),
            TestUserIdentityRepository::default(),
            google_client_for_test(),
        );

        let error = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: String::new(),
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect_err("missing state should fail");

        assert!(matches!(error, LoginServiceError::MissingOAuthState));
    }

    #[tokio::test]
    async fn complete_google_login_rejects_invalid_oauth_state() {
        let login_service = login_service_for_test(
            TestUserRepository::default(),
            TestUserIdentityRepository::default(),
            google_client_for_test(),
        );

        let error = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: "invalid-state".to_owned(),
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect_err("invalid state should fail");

        assert!(matches!(
            error,
            LoginServiceError::GoogleOAuthState(GoogleOAuthStateError::InvalidState)
        ));
    }

    #[tokio::test]
    async fn complete_google_login_returns_binding_conflict_when_email_is_already_bound() {
        let user_repository = TestUserRepository::seeded(seeded_user());
        let user_identity_repository =
            TestUserIdentityRepository::seeded(seeded_conflicting_identity());
        let login_service = login_service_for_test(
            user_repository,
            user_identity_repository,
            google_client_for_test(),
        );
        let authorization_request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        let error = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: authorization_request.state,
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect_err("conflicting identity should fail");

        assert!(matches!(
            error,
            LoginServiceError::IdentityBindingConflict { .. }
        ));
    }

    #[tokio::test]
    async fn complete_google_login_returns_oauth_error_when_exchange_fails() {
        let login_service = login_service_for_test(
            TestUserRepository::default(),
            TestUserIdentityRepository::default(),
            google_client_for_exchange_error(),
        );
        let authorization_request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        let error = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: authorization_request.state,
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect_err("oauth exchange failure should bubble up");

        assert!(matches!(error, LoginServiceError::GoogleOAuth(_)));
    }

    #[tokio::test]
    async fn complete_google_login_returns_oauth_error_when_profile_fetch_fails() {
        let login_service = login_service_for_test(
            TestUserRepository::default(),
            TestUserIdentityRepository::default(),
            google_client_for_profile_error(),
        );
        let authorization_request = login_service
            .build_google_authorization_request()
            .expect("authorization request should build");

        let error = login_service
            .complete_google_login(GoogleLoginCallbackCommand {
                authorization_code: "google-code-123".to_owned(),
                oauth_state: authorization_request.state,
                user_agent: "kiro-test-agent".to_owned(),
            })
            .await
            .expect_err("profile fetch failure should bubble up");

        assert!(matches!(error, LoginServiceError::GoogleOAuth(_)));
    }
}
