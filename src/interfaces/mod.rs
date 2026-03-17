use std::time::Instant;

use axum::Extension;
use axum::Router;
use axum::middleware as axum_middleware;
use axum::routing::{get, post};

use crate::application::AppServices;
use crate::config::AppConfig;
use crate::interfaces::controller::{auth as auth_controller, health as health_controller};
use crate::interfaces::middleware::authentication::{require_access_token, require_refresh_token};
use crate::interfaces::middleware::trace_id::RequestTrace;
use crate::interfaces::middleware::trace_id::trace_id_middleware;
use crate::interfaces::response::AppError;

pub mod controller;
pub mod dto;
pub mod middleware;
pub mod response;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub services: AppServices,
    pub started_at: Instant,
}

impl AppState {
    pub fn new(config: AppConfig, services: AppServices) -> Self {
        Self {
            config,
            services,
            started_at: Instant::now(),
        }
    }
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .nest("/health", build_health_routes())
        .nest("/auth", build_auth_routes(state.clone()))
        .fallback(not_found)
        .layer(axum_middleware::from_fn(trace_id_middleware))
        .with_state(state)
}

fn build_health_routes() -> Router<AppState> {
    Router::new()
        .route("/live", get(health_controller::live))
        .route("/ready", get(health_controller::ready))
}

fn build_auth_routes(state: AppState) -> Router<AppState> {
    let public_routes = Router::new()
        .route(
            "/google/authorization-url",
            get(auth_controller::google_authorization_url),
        )
        .route("/google/callback", get(auth_controller::google_callback));

    let access_routes = Router::new()
        .route("/me", get(auth_controller::current_user))
        .route("/protected", get(auth_controller::protected_session))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            require_access_token,
        ));

    let refresh_routes = Router::new()
        .route("/refresh", post(auth_controller::refresh_session))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            require_refresh_token,
        ));

    let session_routes = Router::new()
        .route("/logout", post(auth_controller::logout_session))
        .route_layer(axum_middleware::from_fn_with_state(
            state.clone(),
            require_refresh_token,
        ))
        .route_layer(axum_middleware::from_fn_with_state(
            state,
            require_access_token,
        ));

    Router::new()
        .merge(public_routes)
        .merge(access_routes)
        .merge(refresh_routes)
        .merge(session_routes)
}

async fn not_found(Extension(request_trace): Extension<RequestTrace>) -> AppError {
    AppError::not_found(request_trace.trace_id().to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::{Body, to_bytes};
    use axum::http::header::HeaderValue;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use time::OffsetDateTime;
    use tower::ServiceExt;

    use super::build_router;
    use crate::application::AppServices;
    use crate::application::account::AccountService;
    use crate::application::auth::AuthService;
    use crate::application::health::HealthService;
    use crate::application::login::LoginService;
    use crate::application::user_identity::UserIdentityService;
    use crate::config::AppConfig;
    use crate::domain::account::{User, UserStatus};
    use crate::infrastructure::auth::google::{
        GoogleOAuthClient, GoogleOAuthClientBuilder, GoogleTokenResponse, GoogleUserProfile,
    };
    use crate::infrastructure::auth::google_state::GoogleOAuthStateServiceBuilder;
    use crate::infrastructure::persistence::in_memory::accounts::user_identity_repository::InMemoryUserIdentityRepository;
    use crate::infrastructure::persistence::in_memory::accounts::user_repository::InMemoryUserRepository;
    use crate::infrastructure::{BootstrapResources, DependencyHealth, ReadinessState};
    use crate::interfaces::AppState;
    use crate::interfaces::middleware::authentication::REFRESH_TOKEN_HEADER_NAME;
    use crate::interfaces::middleware::trace_id::TRACE_ID_HEADER_NAME;

    fn test_config() -> AppConfig {
        AppConfig::from_env_map_for_test(&[
            ("RUNTIME_ENV", "test"),
            ("SERVICE_NAME", "kiro-test"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
            ("BLACKLIST_MODE", "memory"),
        ])
    }

    fn google_test_config() -> AppConfig {
        AppConfig::from_env_map_for_test(&[
            ("RUNTIME_ENV", "test"),
            ("SERVICE_NAME", "kiro-test"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
            ("BLACKLIST_MODE", "memory"),
            ("GOOGLE_AUTH_ENABLED", "true"),
            ("GOOGLE_CLIENT_ID", "google-client-id"),
            ("GOOGLE_CLIENT_SECRET", "google-client-secret"),
            (
                "GOOGLE_REDIRECT_URI",
                "http://localhost:3000/auth/google/callback",
            ),
        ])
    }

    fn ready_test_state() -> AppState {
        let config = test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let services = AppServices::new(
            None,
            AuthService::new(resources.jwt_service, resources.token_blacklist_service),
            HealthService::new(resources.readiness),
            None,
            None,
        );
        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn not_ready_test_state() -> AppState {
        let config = test_config();
        let resources = BootstrapResources::not_ready_for_test(
            &config,
            "postgres unavailable",
            "redis unavailable",
        );
        let services = AppServices::new(
            None,
            AuthService::new(resources.jwt_service, resources.token_blacklist_service),
            HealthService::new(resources.readiness),
            None,
            None,
        );
        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn google_enabled_test_state() -> AppState {
        let config = google_test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let auth_service =
            AuthService::new(resources.jwt_service, resources.token_blacklist_service);
        let health_service = HealthService::new(resources.readiness);
        let login_service = Some(LoginService::new(
            None,
            auth_service.clone(),
            GoogleOAuthClientBuilder::new(config.auth.google.clone())
                .build()
                .expect("google client should build"),
            GoogleOAuthStateServiceBuilder::new(config.auth.clone())
                .build()
                .expect("google state service should build"),
            None,
        ));
        let services = AppServices::new(None, auth_service, health_service, login_service, None);

        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn google_oauth_client_for_test(config: &AppConfig) -> GoogleOAuthClient {
        GoogleOAuthClient::for_test(
            config.auth.google.clone(),
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

    fn google_oauth_exchange_error_client_for_test(config: &AppConfig) -> GoogleOAuthClient {
        GoogleOAuthClient::for_test_exchange_error(
            config.auth.google.clone(),
            502,
            "google exchange failed",
        )
    }

    fn google_oauth_state_service_for_test(
        config: &AppConfig,
    ) -> crate::infrastructure::auth::google_state::GoogleOAuthStateService {
        GoogleOAuthStateServiceBuilder::new(config.auth.clone())
            .build()
            .expect("google state service should build")
            .expect("google state service should exist")
    }

    fn google_callback_success_test_state() -> AppState {
        let config = google_test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let auth_service =
            AuthService::new(resources.jwt_service, resources.token_blacklist_service);
        let health_service = HealthService::new(resources.readiness);
        let account_service = AccountService::new(InMemoryUserRepository::default());
        let user_identity_service =
            UserIdentityService::new(InMemoryUserIdentityRepository::default());
        let login_service = LoginService::new(
            Some(account_service.clone()),
            auth_service.clone(),
            Some(google_oauth_client_for_test(&config)),
            Some(google_oauth_state_service_for_test(&config)),
            Some(user_identity_service),
        );
        let services = AppServices::new(None, auth_service, health_service, None, None)
            .with_test_account(account_service)
            .with_test_login(login_service);

        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn current_user_test_state(user: User) -> AppState {
        let config = test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let auth_service =
            AuthService::new(resources.jwt_service, resources.token_blacklist_service);
        let health_service = HealthService::new(resources.readiness);
        let account_service = AccountService::new(InMemoryUserRepository::seeded(user));
        let services = AppServices::new(None, auth_service, health_service, None, None)
            .with_test_account(account_service);

        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn google_callback_conflict_test_state() -> AppState {
        let config = google_test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let auth_service =
            AuthService::new(resources.jwt_service, resources.token_blacklist_service);
        let health_service = HealthService::new(resources.readiness);
        let account_service =
            AccountService::new(InMemoryUserRepository::seeded(seeded_conflict_user()));
        let user_identity_service = UserIdentityService::new(
            InMemoryUserIdentityRepository::seeded(seeded_conflict_identity()),
        );
        let login_service = LoginService::new(
            Some(account_service.clone()),
            auth_service.clone(),
            Some(google_oauth_client_for_test(&config)),
            Some(google_oauth_state_service_for_test(&config)),
            Some(user_identity_service),
        );
        let services = AppServices::new(None, auth_service, health_service, None, None)
            .with_test_account(account_service)
            .with_test_login(login_service);

        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn google_callback_exchange_failure_test_state() -> AppState {
        let config = google_test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let auth_service =
            AuthService::new(resources.jwt_service, resources.token_blacklist_service);
        let health_service = HealthService::new(resources.readiness);
        let account_service = AccountService::new(InMemoryUserRepository::default());
        let user_identity_service =
            UserIdentityService::new(InMemoryUserIdentityRepository::default());
        let login_service = LoginService::new(
            Some(account_service.clone()),
            auth_service.clone(),
            Some(google_oauth_exchange_error_client_for_test(&config)),
            Some(google_oauth_state_service_for_test(&config)),
            Some(user_identity_service),
        );
        let services = AppServices::new(None, auth_service, health_service, None, None)
            .with_test_account(account_service)
            .with_test_login(login_service);

        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
    }

    fn seeded_current_user() -> User {
        let now = OffsetDateTime::now_utc();
        User {
            id: 42,
            user_code: "user_current_42".to_owned(),
            email: Some("hello@example.com".to_owned()),
            email_normalized: Some("hello@example.com".to_owned()),
            display_name: Some("Hello User".to_owned()),
            avatar_url: Some("https://example.com/avatar.png".to_owned()),
            locale: "en-US".to_owned(),
            time_zone: "Asia/Shanghai".to_owned(),
            status: UserStatus::Active,
            last_login_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    fn seeded_conflict_user() -> User {
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

    fn seeded_conflict_identity() -> crate::domain::user_identity::UserIdentity {
        let now = OffsetDateTime::now_utc();
        crate::domain::user_identity::UserIdentity {
            id: 9,
            identity_code: "identity_conflict".to_owned(),
            user_id: 99,
            provider: crate::domain::user_identity::IdentityProvider::Google,
            provider_user_id: "other-google-subject".to_owned(),
            provider_email: Some("hello@example.com".to_owned()),
            provider_email_normalized: Some("hello@example.com".to_owned()),
            profile: serde_json::json!({ "sub": "other-google-subject" }),
            last_authenticated_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    async fn perform_google_login(state: &AppState) -> Value {
        let authorization_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/google/authorization-url")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("authorization request should succeed");
        let authorization_body = to_bytes(authorization_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let authorization_json = serde_json::from_slice::<Value>(&authorization_body)
            .expect("body should be valid json");
        let oauth_state = authorization_json["data"]["state"]
            .as_str()
            .expect("state should be string");
        let callback_uri =
            format!("/auth/google/callback?code=google-code-123&state={oauth_state}");

        let callback_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri(callback_uri)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("callback request should succeed");

        assert_eq!(callback_response.status(), StatusCode::OK);

        let callback_body = to_bytes(callback_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        serde_json::from_slice::<Value>(&callback_body).expect("body should be valid json")
    }

    #[tokio::test]
    async fn live_health_route_returns_ok() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let has_trace_id_header = response.headers().contains_key(&TRACE_ID_HEADER_NAME);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "ok");
        assert_eq!(json["data"]["service"], "kiro-test");
        assert!(has_trace_id_header);
    }

    #[tokio::test]
    async fn google_authorization_url_route_returns_url_state_and_nonce_when_enabled() {
        let app = build_router(google_enabled_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/google/authorization-url")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert!(json["data"]["authorization_url"].as_str().is_some());
        assert!(json["data"]["state"].as_str().is_some());
        assert!(json["data"]["nonce"].as_str().is_some());
        assert!(
            json["data"]["authorization_url"]
                .as_str()
                .expect("authorization_url should be a string")
                .contains("client_id=google-client-id")
        );
    }

    #[tokio::test]
    async fn google_authorization_url_route_returns_service_unavailable_when_disabled() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/google/authorization-url")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "google_login_unavailable");
    }

    #[tokio::test]
    async fn google_callback_route_returns_bad_request_when_code_is_missing() {
        let app = build_router(google_enabled_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/google/callback")
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "missing_authorization_code");
    }

    #[tokio::test]
    async fn google_callback_route_returns_service_unavailable_when_disabled() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/google/callback?code=google-code-123")
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "google_login_unavailable");
    }

    #[tokio::test]
    async fn google_callback_route_returns_bad_request_when_state_is_missing() {
        let app = build_router(google_enabled_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/google/callback?code=google-code-123")
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "missing_google_state");
    }

    #[tokio::test]
    async fn google_callback_route_returns_bad_request_when_state_is_invalid() {
        let app = build_router(google_callback_success_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/google/callback?code=google-code-123&state=invalid-google-state")
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "invalid_google_state");
    }

    #[tokio::test]
    async fn google_callback_route_returns_token_pair_when_login_succeeds() {
        let state = google_callback_success_test_state();
        let json = perform_google_login(&state).await;

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["provider"], "google");
        assert_eq!(json["data"]["is_new_user"], true);
        assert!(json["data"]["user_code"].as_str().is_some());
        assert!(json["data"]["identity_code"].as_str().is_some());

        let access_token = json["data"]["access_token"]
            .as_str()
            .expect("access token should be string");
        let refresh_token = json["data"]["refresh_token"]
            .as_str()
            .expect("refresh token should be string");
        let user_code = json["data"]["user_code"]
            .as_str()
            .expect("user code should be string");

        let validated_access = state
            .services
            .auth
            .validate_access_token(access_token)
            .expect("access token should validate");
        let validated_refresh = state
            .services
            .auth
            .validate_refresh_token(refresh_token)
            .expect("refresh token should validate");

        assert_eq!(validated_access.subject, user_code);
        assert_eq!(validated_refresh.subject, user_code);
    }

    #[tokio::test]
    async fn google_login_session_can_access_protected_and_current_user_endpoints() {
        let state = google_callback_success_test_state();
        let login_json = perform_google_login(&state).await;
        let access_token = login_json["data"]["access_token"]
            .as_str()
            .expect("access token should be string");
        let user_code = login_json["data"]["user_code"]
            .as_str()
            .expect("user code should be string");

        let protected_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("protected request should succeed");

        assert_eq!(protected_response.status(), StatusCode::OK);
        let protected_body = to_bytes(protected_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let protected_json =
            serde_json::from_slice::<Value>(&protected_body).expect("body should be valid json");
        assert_eq!(protected_json["success"], true);
        assert_eq!(protected_json["data"]["subject"], user_code);

        let me_response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", format!("Bearer {access_token}"))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("current user request should succeed");

        assert_eq!(me_response.status(), StatusCode::OK);
        let me_body = to_bytes(me_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let me_json = serde_json::from_slice::<Value>(&me_body).expect("body should be valid json");
        assert_eq!(me_json["success"], true);
        assert_eq!(me_json["data"]["user_code"], user_code);
        assert_eq!(me_json["data"]["email"], "hello@example.com");
    }

    #[tokio::test]
    async fn refresh_route_revokes_old_refresh_token_and_keeps_old_access_token_usable_until_expiry()
     {
        let state = google_callback_success_test_state();
        let login_json = perform_google_login(&state).await;
        let old_access_token = login_json["data"]["access_token"]
            .as_str()
            .expect("access token should be string")
            .to_owned();
        let old_refresh_token = login_json["data"]["refresh_token"]
            .as_str()
            .expect("refresh token should be string")
            .to_owned();

        let refresh_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, old_refresh_token.clone())
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("refresh request should succeed");

        assert_eq!(refresh_response.status(), StatusCode::OK);
        let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let refresh_json =
            serde_json::from_slice::<Value>(&refresh_body).expect("body should be valid json");
        let new_access_token = refresh_json["data"]["access_token"]
            .as_str()
            .expect("access token should be string");
        let new_refresh_token = refresh_json["data"]["refresh_token"]
            .as_str()
            .expect("refresh token should be string");

        let reuse_old_refresh_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, old_refresh_token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("refresh request should succeed");

        assert_eq!(
            reuse_old_refresh_response.status(),
            StatusCode::UNAUTHORIZED
        );
        let reuse_old_refresh_body = to_bytes(reuse_old_refresh_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let reuse_old_refresh_json = serde_json::from_slice::<Value>(&reuse_old_refresh_body)
            .expect("body should be valid json");
        assert_eq!(reuse_old_refresh_json["error"]["code"], "token_revoked");

        let old_access_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {old_access_token}"))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("protected request should succeed");
        assert_eq!(old_access_response.status(), StatusCode::OK);

        let new_access_response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {new_access_token}"))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("protected request should succeed");
        assert_eq!(new_access_response.status(), StatusCode::OK);
        assert!(!new_refresh_token.is_empty());
    }

    #[tokio::test]
    async fn logout_route_revokes_rotated_tokens_after_google_login() {
        let state = google_callback_success_test_state();
        let login_json = perform_google_login(&state).await;
        let old_refresh_token = login_json["data"]["refresh_token"]
            .as_str()
            .expect("refresh token should be string")
            .to_owned();

        let refresh_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, old_refresh_token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("refresh request should succeed");

        assert_eq!(refresh_response.status(), StatusCode::OK);
        let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let refresh_json =
            serde_json::from_slice::<Value>(&refresh_body).expect("body should be valid json");
        let current_access_token = refresh_json["data"]["access_token"]
            .as_str()
            .expect("access token should be string")
            .to_owned();
        let current_refresh_token = refresh_json["data"]["refresh_token"]
            .as_str()
            .expect("refresh token should be string")
            .to_owned();

        let logout_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header("authorization", format!("Bearer {current_access_token}"))
                    .header(&REFRESH_TOKEN_HEADER_NAME, current_refresh_token.clone())
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("logout request should succeed");

        assert_eq!(logout_response.status(), StatusCode::OK);

        let protected_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {current_access_token}"))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("protected request should succeed");
        assert_eq!(protected_response.status(), StatusCode::UNAUTHORIZED);
        let protected_body = to_bytes(protected_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let protected_json =
            serde_json::from_slice::<Value>(&protected_body).expect("body should be valid json");
        assert_eq!(protected_json["error"]["code"], "token_revoked");

        let refresh_again_response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, current_refresh_token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("refresh request should succeed");
        assert_eq!(refresh_again_response.status(), StatusCode::UNAUTHORIZED);
        let refresh_again_body = to_bytes(refresh_again_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let refresh_again_json = serde_json::from_slice::<Value>(&refresh_again_body)
            .expect("body should be valid json");
        assert_eq!(refresh_again_json["error"]["code"], "token_revoked");
    }

    #[tokio::test]
    async fn google_callback_route_returns_conflict_when_identity_binding_conflicts() {
        let state = google_callback_conflict_test_state();
        let authorization_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/google/authorization-url")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("authorization request should succeed");
        let authorization_body = to_bytes(authorization_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let authorization_json = serde_json::from_slice::<Value>(&authorization_body)
            .expect("body should be valid json");
        let oauth_state = authorization_json["data"]["state"]
            .as_str()
            .expect("state should be string");
        let callback_uri =
            format!("/auth/google/callback?code=google-code-123&state={oauth_state}");

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri(callback_uri)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "identity_binding_conflict");
    }

    #[tokio::test]
    async fn google_callback_route_returns_bad_gateway_when_google_exchange_fails() {
        let state = google_callback_exchange_failure_test_state();
        let authorization_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/google/authorization-url")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("authorization request should succeed");
        let authorization_body = to_bytes(authorization_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let authorization_json = serde_json::from_slice::<Value>(&authorization_body)
            .expect("body should be valid json");
        let oauth_state = authorization_json["data"]["state"]
            .as_str()
            .expect("state should be string");
        let callback_uri =
            format!("/auth/google/callback?code=google-code-123&state={oauth_state}");

        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri(callback_uri)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "google_oauth_exchange_failed");
    }

    #[tokio::test]
    async fn ready_health_route_returns_ok() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let has_trace_id_header = response.headers().contains_key(&TRACE_ID_HEADER_NAME);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "ready");
        assert_eq!(json["data"]["checks"]["http_server"]["status"], "ok");
        assert_eq!(json["data"]["checks"]["postgres"]["status"], "ok");
        assert_eq!(json["data"]["checks"]["redis"]["status"], "ok");
        assert!(has_trace_id_header);
    }

    #[tokio::test]
    async fn ready_health_route_returns_service_unavailable_when_dependencies_fail() {
        let app = build_router(not_ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let has_trace_id_header = response.headers().contains_key(&TRACE_ID_HEADER_NAME);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "not_ready");
        assert_eq!(json["data"]["checks"]["postgres"]["status"], "error");
        assert_eq!(json["data"]["checks"]["redis"]["status"], "error");
        assert!(has_trace_id_header);
    }

    #[tokio::test]
    async fn fallback_returns_uniform_error_response_and_trace_id() {
        let app = build_router(ready_test_state());

        let request_trace_id = "trace_from_request_123";
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/missing")
                    .header(
                        &TRACE_ID_HEADER_NAME,
                        HeaderValue::from_static("trace_from_request_123"),
                    )
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get(&TRACE_ID_HEADER_NAME).unwrap(),
            request_trace_id
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "route_not_found");
        assert_eq!(json["error"]["trace_id"], request_trace_id);
    }

    #[tokio::test]
    async fn middleware_generates_trace_id_when_missing() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        let trace_id = response
            .headers()
            .get(&TRACE_ID_HEADER_NAME)
            .and_then(|value| value.to_str().ok())
            .expect("trace id header should exist");

        assert!(!trace_id.is_empty());
    }

    #[tokio::test]
    async fn protected_route_allows_valid_access_token() {
        let state = ready_test_state();
        let token = state
            .services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["subject"], "user_42");
    }

    #[tokio::test]
    async fn current_user_route_returns_authenticated_user() {
        let state = current_user_test_state(seeded_current_user());
        let token = state
            .services
            .auth
            .issue_access_token("user_current_42", "kiro-test-agent")
            .expect("access token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["user_code"], "user_current_42");
        assert_eq!(json["data"]["email"], "hello@example.com");
        assert_eq!(json["data"]["display_name"], "Hello User");
        assert_eq!(json["data"]["status"], "active");
        assert_eq!(json["data"]["time_zone"], "Asia/Shanghai");
        assert!(json["data"]["last_login_at"].as_i64().is_some());
    }

    #[tokio::test]
    async fn current_user_route_returns_service_unavailable_when_account_service_is_missing() {
        let state = ready_test_state();
        let token = state
            .services
            .auth
            .issue_access_token("user_current_42", "kiro-test-agent")
            .expect("access token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/me")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "current_user_unavailable");
    }

    #[tokio::test]
    async fn protected_route_rejects_missing_bearer_token() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "missing_bearer_token");
    }

    #[tokio::test]
    async fn protected_route_rejects_refresh_token() {
        let state = ready_test_state();
        let token = state
            .services
            .auth
            .issue_refresh_token("user_42", "kiro-test-agent")
            .expect("refresh token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "invalid_token_kind");
    }

    #[tokio::test]
    async fn protected_route_rejects_user_agent_mismatch() {
        let state = ready_test_state();
        let token = state
            .services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "other-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "user_agent_mismatch");
    }

    #[tokio::test]
    async fn protected_route_rejects_revoked_token() {
        let state = ready_test_state();
        let token = state
            .services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        state
            .services
            .auth
            .revoke_access_token(&token.jti, token.expires_at)
            .await
            .expect("token revoke should succeed");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "token_revoked");
    }

    #[tokio::test]
    async fn protected_route_returns_service_unavailable_when_blacklist_backend_is_unavailable() {
        let config = AppConfig::from_env_map_for_test(&[
            ("RUNTIME_ENV", "test"),
            ("SERVICE_NAME", "kiro-test"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
            ("BLACKLIST_MODE", "redis"),
        ]);
        let resources = BootstrapResources {
            google_oauth_client: None,
            google_oauth_state_service: None,
            postgres_pool: None,
            redis_client: None,
            jwt_service: crate::infrastructure::auth::jwt::JwtServiceBuilder::new(
                config.auth.clone(),
            )
            .build()
            .expect("jwt service should build"),
            token_blacklist_service:
                crate::infrastructure::auth::blacklist::TokenBlacklistServiceBuilder::new(
                    config.auth.blacklist_mode,
                )
                .with_redis_client(
                    None,
                    std::time::Duration::from_secs(config.redis.connect_timeout_seconds),
                )
                .with_key_prefix(config.redis.key_prefix.clone())
                .build(),
            readiness: Arc::new(ReadinessState {
                postgres: DependencyHealth::ready(),
                redis: DependencyHealth::not_ready("redis unavailable"),
            }),
        };
        let services = AppServices::new(
            None,
            AuthService::new(resources.jwt_service, resources.token_blacklist_service),
            HealthService::new(resources.readiness),
            None,
            None,
        );
        let token = services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        let state = AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        };
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header("authorization", format!("Bearer {}", token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], false);
        assert_eq!(json["error"]["code"], "blacklist_unavailable");
    }

    #[tokio::test]
    async fn refresh_route_rotates_token_pair() {
        let state = ready_test_state();
        let refresh_token = state
            .services
            .auth
            .issue_refresh_token("user_42", "kiro-test-agent")
            .expect("refresh token should issue");
        let previous_refresh_jti = refresh_token.jti.clone();
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, refresh_token.token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        let new_refresh_token = json["data"]["refresh_token"]
            .as_str()
            .expect("refresh token should be string");
        assert!(
            state
                .services
                .auth
                .is_refresh_token_revoked(&previous_refresh_jti)
                .await
                .expect("blacklist lookup should succeed")
        );
        let validated = state
            .services
            .auth
            .validate_refresh_token(new_refresh_token)
            .expect("new refresh token should validate");
        assert_eq!(validated.subject, "user_42");
    }

    #[tokio::test]
    async fn refresh_route_rejects_missing_refresh_token_header() {
        let app = build_router(ready_test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "missing_refresh_token");
    }

    #[tokio::test]
    async fn refresh_route_rejects_access_token_in_refresh_header() {
        let state = ready_test_state();
        let access_token = state
            .services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, access_token.token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "invalid_token_kind");
    }

    #[tokio::test]
    async fn refresh_route_rejects_revoked_refresh_token() {
        let state = ready_test_state();
        let refresh_token = state
            .services
            .auth
            .issue_refresh_token("user_42", "kiro-test-agent")
            .expect("refresh token should issue");
        state
            .services
            .auth
            .revoke_refresh_token(&refresh_token.jti, refresh_token.expires_at)
            .await
            .expect("token revoke should succeed");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(&REFRESH_TOKEN_HEADER_NAME, refresh_token.token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "token_revoked");
    }

    #[tokio::test]
    async fn logout_route_revokes_access_and_refresh_tokens() {
        let state = ready_test_state();
        let token_pair = state
            .services
            .auth
            .issue_token_pair("user_42", "kiro-test-agent")
            .expect("token pair should issue");
        let access_jti = token_pair.access_token.jti.clone();
        let refresh_jti = token_pair.refresh_token.jti.clone();
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header(
                        "authorization",
                        format!("Bearer {}", token_pair.access_token.token),
                    )
                    .header(&REFRESH_TOKEN_HEADER_NAME, token_pair.refresh_token.token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["subject"], "user_42");
        assert_eq!(json["data"]["access_token_revoked"], true);
        assert_eq!(json["data"]["refresh_token_revoked"], true);
        assert!(
            state
                .services
                .auth
                .is_access_token_revoked(&access_jti)
                .await
                .expect("blacklist lookup should succeed")
        );
        assert!(
            state
                .services
                .auth
                .is_refresh_token_revoked(&refresh_jti)
                .await
                .expect("blacklist lookup should succeed")
        );
    }

    #[tokio::test]
    async fn logout_route_rejects_missing_refresh_token_header() {
        let state = ready_test_state();
        let access_token = state
            .services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        let app = build_router(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header("authorization", format!("Bearer {}", access_token.token))
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "missing_refresh_token");
    }

    #[tokio::test]
    async fn logout_route_rejects_subject_mismatch() {
        let state = ready_test_state();
        let access_token = state
            .services
            .auth
            .issue_access_token("user_42", "kiro-test-agent")
            .expect("access token should issue");
        let refresh_token = state
            .services
            .auth
            .issue_refresh_token("user_99", "kiro-test-agent")
            .expect("refresh token should issue");
        let access_jti = access_token.jti.clone();
        let refresh_jti = refresh_token.jti.clone();
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header("authorization", format!("Bearer {}", access_token.token))
                    .header(&REFRESH_TOKEN_HEADER_NAME, refresh_token.token)
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let json = serde_json::from_slice::<Value>(&body).expect("body should be valid json");

        assert_eq!(json["error"]["code"], "token_subject_mismatch");
        assert!(
            !state
                .services
                .auth
                .is_access_token_revoked(&access_jti)
                .await
                .expect("blacklist lookup should succeed")
        );
        assert!(
            !state
                .services
                .auth
                .is_refresh_token_revoked(&refresh_jti)
                .await
                .expect("blacklist lookup should succeed")
        );
    }

    #[tokio::test]
    async fn revoked_tokens_are_rejected_after_logout() {
        let state = ready_test_state();
        let token_pair = state
            .services
            .auth
            .issue_token_pair("user_42", "kiro-test-agent")
            .expect("token pair should issue");

        let logout_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/logout")
                    .header(
                        "authorization",
                        format!("Bearer {}", token_pair.access_token.token),
                    )
                    .header(
                        &REFRESH_TOKEN_HEADER_NAME,
                        token_pair.refresh_token.token.clone(),
                    )
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("logout request should succeed");

        assert_eq!(logout_response.status(), StatusCode::OK);

        let protected_response = build_router(state.clone())
            .oneshot(
                Request::builder()
                    .uri("/auth/protected")
                    .header(
                        "authorization",
                        format!("Bearer {}", token_pair.access_token.token),
                    )
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("protected request should succeed");

        assert_eq!(protected_response.status(), StatusCode::UNAUTHORIZED);
        let protected_body = to_bytes(protected_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let protected_json =
            serde_json::from_slice::<Value>(&protected_body).expect("body should be valid json");
        assert_eq!(protected_json["error"]["code"], "token_revoked");

        let refresh_response = build_router(state)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/refresh")
                    .header(
                        &REFRESH_TOKEN_HEADER_NAME,
                        token_pair.refresh_token.token.clone(),
                    )
                    .header("user-agent", "kiro-test-agent")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("refresh request should succeed");

        assert_eq!(refresh_response.status(), StatusCode::UNAUTHORIZED);
        let refresh_body = to_bytes(refresh_response.into_body(), usize::MAX)
            .await
            .expect("body should read");
        let refresh_json =
            serde_json::from_slice::<Value>(&refresh_body).expect("body should be valid json");
        assert_eq!(refresh_json["error"]["code"], "token_revoked");
    }
}
