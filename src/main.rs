use anyhow::Result;
use dotenv::dotenv;
use tracing::info;

mod application;
mod config;
mod domain;
mod infrastructure;
mod interfaces;
mod server;
mod telemetry;

use crate::application::auth::AuthService;
use crate::application::health::HealthService;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = config::AppConfig::from_env()?;
    telemetry::init_tracing(&config)?;
    let resources = infrastructure::bootstrap(&config).await?;
    let auth_service = AuthService::new(resources.jwt_service, resources.token_blacklist_service);
    let health_service = HealthService::new(resources.readiness);
    let services = application::AppServices::new(auth_service, health_service);
    let state = interfaces::AppState::new(config.clone(), services);

    info!(
        service_name = %config.service.name,
        runtime_env = %config.service.runtime_env,
        bind_address = %config.socket_addr(),
        "application bootstrap completed"
    );

    server::run_http_server(state).await
}
