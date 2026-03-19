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

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct TelemetryConfig {
    pub name: String,
    pub endpoint: String,
    pub level: String,
}

#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub telemetry: TelemetryConfig,
    #[allow(dead_code)]
    pub jwt: JwtConfig,
    #[allow(dead_code)]
    pub google: GoogleConfig,
}

pub fn load_config(path: &str) -> Result<AppConfig> {
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read config file: {path}"))?;

    let config =
        toml::from_str(&content).with_context(|| format!("failed to parse config file: {path}"))?;

    Ok(config)
}
