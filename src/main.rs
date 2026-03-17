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

use crate::application::account::AccountService;
use crate::application::auth::AuthService;
use crate::application::health::HealthService;
use crate::application::login::DefaultLoginService;
use crate::application::user_identity::UserIdentityService;
use crate::infrastructure::persistence::postgres::accounts::user_identity_repository::PostgresUserIdentityRepository;
use crate::infrastructure::persistence::postgres::accounts::user_repository::PostgresUserRepository;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let config = config::AppConfig::from_env()?;

    telemetry::init_tracing(&config)?;

    let resources = infrastructure::bootstrap(&config).await?;
    let account_service = resources
        .postgres_pool
        .clone()
        .map(PostgresUserRepository::new)
        .map(AccountService::new);

    let auth_service = AuthService::new(resources.jwt_service, resources.token_blacklist_service);
    let health_service = HealthService::new(resources.readiness);
    let user_identity_service = resources
        .postgres_pool
        .clone()
        .map(PostgresUserIdentityRepository::new)
        .map(UserIdentityService::new);
    let login_service = DefaultLoginService::new(
        account_service.clone(),
        auth_service.clone(),
        resources.google_oauth_client,
        resources.google_oauth_state_service,
        user_identity_service.clone(),
    );
    let login_service = Some(login_service);
    let services = application::AppServices::new(
        account_service,
        auth_service,
        health_service,
        login_service,
        user_identity_service,
    );

    let state = interfaces::AppState::new(config.clone(), services);

    info!(
        service_name = %config.service.name,
        runtime_env = %config.service.runtime_env,
        bind_address = %config.socket_addr(),
        "application bootstrap completed"
    );

    server::run_http_server(state).await
}
