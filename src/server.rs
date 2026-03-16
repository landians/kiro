use std::time::Duration;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::interfaces;
use crate::interfaces::AppState;

pub async fn run_http_server(state: AppState) -> Result<()> {
    let bind_address = state.config.socket_addr();
    let listener = TcpListener::bind(bind_address)
        .await
        .with_context(|| format!("failed to bind HTTP listener on {bind_address}"))?;

    let router = interfaces::build_router(state.clone());

    info!(
        service_name = %state.config.service.name,
        runtime_env = %state.config.service.runtime_env,
        bind_address = %bind_address,
        "HTTP server starting"
    );

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP server terminated with error")?;

    info!("HTTP server stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("ctrl-c signal handler should install successfully");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("terminate signal handler should install successfully")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("received ctrl-c shutdown signal");
        }
        _ = terminate => {
            warn!("received terminate shutdown signal");
        }
    }

    tokio::time::sleep(Duration::from_millis(50)).await;
}
