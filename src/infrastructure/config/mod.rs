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
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub acquire_timeout_seconds: Option<u64>,
    pub idle_timeout_seconds: Option<u64>,
    pub max_lifetime_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RedisConfig {
    pub password: Option<String>,
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryProtocol {
    #[default]
    Grpc,
    Http,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    #[serde(default)]
    pub enabled: bool,
    pub service_name: String,
    pub service_namespace: Option<String>,
    pub service_version: Option<String>,
    pub tracer_name: String,
    pub endpoint: Option<String>,
    #[serde(default)]
    pub protocol: TelemetryProtocol,
    pub level: String,
    pub export_interval_seconds: u64,
    #[serde(default)]
    pub authorization: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtConfig {
    pub issuer: String,
    pub access_secret: String,
    pub refresh_secret: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub http: HttpConfig,
    pub postgres: PgConfig,
    pub redis: RedisConfig,
    pub telemetry: TelemetryConfig,
    pub jwt: JwtConfig,
    pub google: GoogleConfig,
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read config file: {path}"))?;

    let config =
        toml::from_str(&content).with_context(|| format!("failed to parse config file: {path}"))?;

    Ok(config)
}
