use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub env: String,
}

#[derive(Debug, Deserialize)]
pub struct PgConfig {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub password: Option<String>,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    pub name: String,
    pub endpoint: String,
    pub level: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub http: HttpConfig,
    pub postgres: PgConfig,
    pub redis: RedisConfig,
    pub telemetry: TelemetryConfig,
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read config file: {path}"))?;

    let config =
        toml::from_str(&content).with_context(|| format!("failed to parse config file: {path}"))?;

    Ok(config)
}
