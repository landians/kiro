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
    let access_routes = Router::new()
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
    use tower::ServiceExt;

    use super::build_router;
    use crate::application::AppServices;
    use crate::application::auth::AuthService;
    use crate::application::health::HealthService;
    use crate::config::AppConfig;
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

    fn ready_test_state() -> AppState {
        let config = test_config();
        let resources = BootstrapResources::ready_for_test(&config);
        let services = AppServices::new(
            AuthService::new(resources.jwt_service, resources.token_blacklist_service),
            HealthService::new(resources.readiness),
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
            AuthService::new(resources.jwt_service, resources.token_blacklist_service),
            HealthService::new(resources.readiness),
        );
        AppState {
            config,
            services,
            started_at: std::time::Instant::now(),
        }
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
            AuthService::new(resources.jwt_service, resources.token_blacklist_service),
            HealthService::new(resources.readiness),
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
