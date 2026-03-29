use tokio::signal;

use crate::{
    bootstrap::auth::build_auth_logic,
    infrastructure::{
        auth::{AuthServiceBuilder, GoogleAuthServiceBuilder},
        cache::CacheBuilder,
        config,
        persistence::PostgresBuilder,
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
    let c = config::load_config("config.toml").expect("Failed to load configuration");

    let auth_service = AuthServiceBuilder::new(c.jwt.clone())
        .build()
        .expect("Failed to build auth service");
    let google_auth_service = GoogleAuthServiceBuilder::new(c.google.clone())
        .build()
        .expect("Failed to build google auth service");

    let pg_pool = PostgresBuilder::new(c.postgres)
        .build()
        .await
        .expect("Failed to connect postgres");

    let _redis_conn = CacheBuilder::new(c.redis)
        .build()
        .await
        .expect("Failed to connect redis");

    let auth_logic = build_auth_logic(pg_pool.clone());

    let shared_state = SharedState::new(auth_service, google_auth_service, auth_logic);
    let app = build_routes(shared_state);

    let addr = format!("{}:{}", c.http.host, c.http.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind listener");

    println!(
        "[{}] {} listening on {}",
        c.http.env,
        c.http.name,
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Http server terminated with error")
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
            println!("http service exit with graceful shutdown.")
        },
        _ = terminate => {},
    }
}
