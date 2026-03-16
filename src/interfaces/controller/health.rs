use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::infrastructure::DependencyHealth;
use crate::interfaces::AppState;
use crate::interfaces::dto::health::{
    DependencyCheck, HealthLiveResponse, HealthReadyResponse, ReadyChecks,
};
use crate::interfaces::response::ApiSuccess;

pub async fn live(State(state): State<AppState>) -> ApiSuccess<HealthLiveResponse> {
    ApiSuccess::ok(HealthLiveResponse {
        status: "ok",
        service: state.config.service.name.clone(),
        runtime_env: state.config.service.runtime_env.to_string(),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    })
}

pub async fn ready(
    State(state): State<AppState>,
) -> (
    StatusCode,
    Json<crate::interfaces::response::ApiSuccessEnvelope<HealthReadyResponse>>,
) {
    let checks = ReadyChecks {
        http_server: DependencyCheck {
            status: "ok",
            message: None,
        },
        postgres: dependency_check(&state.infrastructure.readiness.postgres),
        redis: dependency_check(&state.infrastructure.readiness.redis),
    };
    let is_ready = state.infrastructure.readiness.is_ready();

    let response = HealthReadyResponse {
        status: if is_ready { "ready" } else { "not_ready" },
        service: state.config.service.name.clone(),
        runtime_env: state.config.service.runtime_env.to_string(),
        checks,
        uptime_seconds: state.started_at.elapsed().as_secs(),
    };

    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(crate::interfaces::response::ApiSuccessEnvelope {
            success: true,
            data: response,
        }),
    )
}

fn dependency_check(health: &DependencyHealth) -> DependencyCheck {
    DependencyCheck {
        status: health.status_label(),
        message: health.reason().map(ToOwned::to_owned),
    }
}
