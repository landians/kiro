use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;

use crate::config::{AppConfig, LogFormat};

pub fn init_tracing(config: &AppConfig) -> Result<()> {
    let env_filter = EnvFilter::try_new(config.observability.log_filter.clone())
        .context("invalid log filter configuration")?;

    match config.observability.log_format {
        LogFormat::Pretty => tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(true)
            .compact()
            .try_init()
            .map_err(|error| {
                anyhow::anyhow!("failed to initialize pretty tracing subscriber: {error}")
            })?,
        LogFormat::Json => tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_target(true)
            .try_init()
            .map_err(|error| {
                anyhow::anyhow!("failed to initialize json tracing subscriber: {error}")
            })?,
    }

    Ok(())
}
