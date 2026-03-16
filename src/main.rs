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

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = config::AppConfig::from_env()?;
    telemetry::init_tracing(&config)?;
    let infrastructure = infrastructure::bootstrap(&config).await?;
    let state = interfaces::AppState::new(config.clone(), infrastructure);

    info!(
        service_name = %config.service.name,
        runtime_env = %config.service.runtime_env,
        bind_address = %config.socket_addr(),
        "application bootstrap completed"
    );

    server::run_http_server(state).await
}
