use std::path::PathBuf;

use tokio::signal;

use crate::{
    bootstrap::{auth::build_auth_logic, user::build_user_logic},
    infrastructure::{
        auth::{AuthServiceBuilder, GoogleAuthServiceBuilder},
        cache::CacheBuilder,
        config,
        persistence::PostgresBuilder,
        telemetry::TelemetryBuilder,
    },
    interfaces::{SharedState, controller::build_routes},
};

mod application;
mod bootstrap;
mod domain;
mod infrastructure;
mod interfaces;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    let config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config.toml");
    let c = config::load_config(config_path.to_string_lossy().as_ref())
        .expect("Failed to load configuration");

    let telemetry = TelemetryBuilder::new(c.telemetry.clone())
        .with_environment(c.http.env.clone())
        .build()
        .expect("Failed to install telemetry");

    let google_auth_service = GoogleAuthServiceBuilder::new(c.google.clone())
        .build()
        .expect("Failed to build google auth service");

    let pg_pool = PostgresBuilder::new(c.postgres)
        .build()
        .await
        .expect("Failed to connect postgres");

    let redis_conn = CacheBuilder::new(c.redis)
        .build()
        .await
        .expect("Failed to connect redis");

    let auth_service = AuthServiceBuilder::new(c.jwt.clone())
        .with_revoked_token_store(redis_conn.clone())
        .build()
        .expect("Failed to build auth service");

    let auth_logic = build_auth_logic(pg_pool.clone());
    let user_logic = build_user_logic(pg_pool.clone());

    let shared_state = SharedState::new(
        auth_service,
        google_auth_service,
        telemetry.http_observability.clone(),
        auth_logic,
        user_logic,
    );
    let app = build_routes(shared_state);

    let addr = format!("{}:{}", c.http.host, c.http.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind listener");

    tracing::info!(
        service_env = %c.http.env,
        service_name = %c.http.name,
        listen_addr = %listener.local_addr().unwrap(),
        "{} listening",
        c.http.name,
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Http server terminated with error");

    telemetry.guard.shutdown();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("http service exit with graceful shutdown.")
        },
        _ = terminate => {},
    }
}
