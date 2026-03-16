use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthLiveResponse {
    pub status: &'static str,
    pub service: String,
    pub runtime_env: String,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct HealthReadyResponse {
    pub status: &'static str,
    pub service: String,
    pub runtime_env: String,
    pub checks: ReadyChecks,
    pub uptime_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct ReadyChecks {
    pub http_server: DependencyCheck,
    pub postgres: DependencyCheck,
    pub redis: DependencyCheck,
}

#[derive(Debug, Serialize)]
pub struct DependencyCheck {
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
