use tokio::signal;

use infrastructure::config;

use crate::{
    infrastructure::{cache::CacheBuilder, persistence::PostgresBuilder},
    interfaces::controller::build_routes,
};

mod application;
mod domain;
mod infrastructure;
mod interfaces;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    let c = config::load_config("./src/config.toml").expect("Failed to load configuration");

    println!("telemetry: {:?}", &c.telemetry);

    let _pg_pool = PostgresBuilder::new(c.postgres)
        .build()
        .await
        .expect("Failed to connect postgres");

    let _redis_conn = CacheBuilder::new(c.redis)
        .build()
        .await
        .expect("Failed to connect redis");

    let app = build_routes();

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
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
