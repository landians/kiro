use std::collections::HashMap;
use std::env;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use anyhow::{Context, Result, anyhow, bail};

const DEFAULT_SERVICE_NAME: &str = "kiro";
const DEFAULT_HTTP_HOST: &str = "127.0.0.1";
const DEFAULT_HTTP_PORT: u16 = 3000;
const DEFAULT_LOG_FILTER: &str = "info";
const DEFAULT_POSTGRES_MAX_CONNECTIONS: u32 = 20;
const DEFAULT_POSTGRES_MIN_CONNECTIONS: u32 = 1;
const DEFAULT_POSTGRES_CONNECT_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_POSTGRES_ACQUIRE_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_POSTGRES_RUN_MIGRATIONS: bool = true;
const DEFAULT_REDIS_CONNECT_TIMEOUT_SECONDS: u64 = 3;
const DEFAULT_REDIS_KEY_PREFIX_SEPARATOR: &str = ":";
const DEFAULT_JWT_ISSUER: &str = "kiro";
const DEFAULT_JWT_AUDIENCE: &str = "kiro-api";
const DEFAULT_JWT_ACCESS_TOKEN_TTL_SECONDS: u64 = 60 * 60 * 2;
const DEFAULT_JWT_REFRESH_TOKEN_TTL_SECONDS: u64 = 60 * 60 * 24 * 15;
const DEFAULT_BLACKLIST_MODE: &str = "redis";
const DEFAULT_GOOGLE_AUTH_ENABLED: bool = false;
const DEFAULT_GOOGLE_AUTHORIZATION_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const DEFAULT_GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_GOOGLE_USER_INFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";
const DEFAULT_GOOGLE_HTTP_TIMEOUT_SECONDS: u64 = 10;
const DEFAULT_GOOGLE_OAUTH_STATE_TTL_SECONDS: u64 = 60 * 10;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub service: ServiceConfig,
    pub http: HttpConfig,
    pub observability: ObservabilityConfig,
    pub postgres: PostgresConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        let vars = env::vars().collect::<HashMap<_, _>>();
        Self::from_map(&vars)
    }

    #[cfg(test)]
    pub fn from_env_map_for_test(pairs: &[(&str, &str)]) -> Self {
        let vars = pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<HashMap<_, _>>();

        Self::from_map(&vars).expect("test config should be valid")
    }

    fn from_map(vars: &HashMap<String, String>) -> Result<Self> {
        let runtime_env = RuntimeEnv::from_str(
            required_var(vars, "RUNTIME_ENV")
                .context("missing required runtime environment configuration")?,
        )
        .context("invalid RUNTIME_ENV value")?;

        let service_name = optional_var(vars, "SERVICE_NAME")
            .unwrap_or(DEFAULT_SERVICE_NAME)
            .to_owned();

        let http_host = optional_var(vars, "HTTP_HOST")
            .unwrap_or(DEFAULT_HTTP_HOST)
            .parse::<IpAddr>()
            .context("invalid HTTP_HOST value")?;

        let http_port = optional_var(vars, "HTTP_PORT")
            .map(str::parse::<u16>)
            .transpose()
            .context("invalid HTTP_PORT value")?
            .unwrap_or(DEFAULT_HTTP_PORT);

        if http_port == 0 {
            bail!("HTTP_PORT must be greater than 0");
        }

        let log_format = optional_var(vars, "LOG_FORMAT")
            .unwrap_or("pretty")
            .parse::<LogFormat>()
            .context("invalid LOG_FORMAT value")?;

        let log_filter = optional_var(vars, "LOG_FILTER")
            .unwrap_or(DEFAULT_LOG_FILTER)
            .to_owned();

        let postgres_url = required_var(vars, "POSTGRES_URL")
            .context("missing required postgres configuration")?
            .to_owned();

        let postgres_max_connections = parse_u32(
            vars,
            "POSTGRES_MAX_CONNECTIONS",
            DEFAULT_POSTGRES_MAX_CONNECTIONS,
        )?;
        let postgres_min_connections = parse_u32(
            vars,
            "POSTGRES_MIN_CONNECTIONS",
            DEFAULT_POSTGRES_MIN_CONNECTIONS,
        )?;
        if postgres_min_connections > postgres_max_connections {
            bail!(
                "POSTGRES_MIN_CONNECTIONS must be less than or equal to POSTGRES_MAX_CONNECTIONS"
            );
        }

        let postgres_connect_timeout_seconds = parse_u64(
            vars,
            "POSTGRES_CONNECT_TIMEOUT_SECONDS",
            DEFAULT_POSTGRES_CONNECT_TIMEOUT_SECONDS,
        )?;
        let postgres_acquire_timeout_seconds = parse_u64(
            vars,
            "POSTGRES_ACQUIRE_TIMEOUT_SECONDS",
            DEFAULT_POSTGRES_ACQUIRE_TIMEOUT_SECONDS,
        )?;
        let postgres_run_migrations = parse_bool(
            vars,
            "POSTGRES_RUN_MIGRATIONS",
            DEFAULT_POSTGRES_RUN_MIGRATIONS,
        )?;

        let redis_url = required_var(vars, "REDIS_URL")
            .context("missing required redis configuration")?
            .to_owned();
        let redis_connect_timeout_seconds = parse_u64(
            vars,
            "REDIS_CONNECT_TIMEOUT_SECONDS",
            DEFAULT_REDIS_CONNECT_TIMEOUT_SECONDS,
        )?;
        let redis_key_prefix = optional_var(vars, "REDIS_KEY_PREFIX")
            .map(str::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "{service_name}{DEFAULT_REDIS_KEY_PREFIX_SEPARATOR}{}",
                    runtime_env.as_str()
                )
            });

        let jwt_issuer = optional_var(vars, "JWT_ISSUER")
            .unwrap_or(DEFAULT_JWT_ISSUER)
            .to_owned();
        let jwt_audience = optional_var(vars, "JWT_AUDIENCE")
            .unwrap_or(DEFAULT_JWT_AUDIENCE)
            .to_owned();
        let jwt_signing_key = required_var(vars, "JWT_SIGNING_KEY")
            .context("missing required jwt signing configuration")?
            .to_owned();
        if jwt_signing_key.len() < 32 {
            bail!("JWT_SIGNING_KEY must be at least 32 characters long");
        }
        let jwt_access_token_ttl_seconds = parse_u64(
            vars,
            "JWT_ACCESS_TOKEN_TTL_SECONDS",
            DEFAULT_JWT_ACCESS_TOKEN_TTL_SECONDS,
        )?;
        let jwt_refresh_token_ttl_seconds = parse_u64(
            vars,
            "JWT_REFRESH_TOKEN_TTL_SECONDS",
            DEFAULT_JWT_REFRESH_TOKEN_TTL_SECONDS,
        )?;
        let blacklist_mode = optional_var(vars, "BLACKLIST_MODE")
            .unwrap_or(DEFAULT_BLACKLIST_MODE)
            .parse::<BlacklistMode>()
            .context("invalid BLACKLIST_MODE value")?;
        let google_auth_enabled =
            parse_bool(vars, "GOOGLE_AUTH_ENABLED", DEFAULT_GOOGLE_AUTH_ENABLED)?;
        let google_client_id = optional_var(vars, "GOOGLE_CLIENT_ID").map(str::to_owned);
        let google_client_secret = optional_var(vars, "GOOGLE_CLIENT_SECRET").map(str::to_owned);
        let google_redirect_uri = optional_var(vars, "GOOGLE_REDIRECT_URI").map(str::to_owned);
        let google_authorization_url = optional_var(vars, "GOOGLE_AUTHORIZATION_URL")
            .unwrap_or(DEFAULT_GOOGLE_AUTHORIZATION_URL)
            .to_owned();
        let google_token_url = optional_var(vars, "GOOGLE_TOKEN_URL")
            .unwrap_or(DEFAULT_GOOGLE_TOKEN_URL)
            .to_owned();
        let google_user_info_url = optional_var(vars, "GOOGLE_USER_INFO_URL")
            .unwrap_or(DEFAULT_GOOGLE_USER_INFO_URL)
            .to_owned();
        let google_http_timeout_seconds = parse_u64(
            vars,
            "GOOGLE_HTTP_TIMEOUT_SECONDS",
            DEFAULT_GOOGLE_HTTP_TIMEOUT_SECONDS,
        )?;
        let google_oauth_state_ttl_seconds = parse_u64(
            vars,
            "GOOGLE_OAUTH_STATE_TTL_SECONDS",
            DEFAULT_GOOGLE_OAUTH_STATE_TTL_SECONDS,
        )?;

        let has_google_credentials = google_client_id.is_some()
            || google_client_secret.is_some()
            || google_redirect_uri.is_some();
        if google_auth_enabled || has_google_credentials {
            if google_client_id.is_none() {
                bail!("GOOGLE_CLIENT_ID is required when google auth is enabled");
            }
            if google_client_secret.is_none() {
                bail!("GOOGLE_CLIENT_SECRET is required when google auth is enabled");
            }
            if google_redirect_uri.is_none() {
                bail!("GOOGLE_REDIRECT_URI is required when google auth is enabled");
            }
        }

        Ok(Self {
            service: ServiceConfig {
                name: service_name,
                runtime_env,
            },
            http: HttpConfig {
                host: http_host,
                port: http_port,
            },
            observability: ObservabilityConfig {
                log_format,
                log_filter,
            },
            postgres: PostgresConfig {
                url: postgres_url,
                max_connections: postgres_max_connections,
                min_connections: postgres_min_connections,
                connect_timeout_seconds: postgres_connect_timeout_seconds,
                acquire_timeout_seconds: postgres_acquire_timeout_seconds,
                run_migrations: postgres_run_migrations,
            },
            redis: RedisConfig {
                url: redis_url,
                connect_timeout_seconds: redis_connect_timeout_seconds,
                key_prefix: redis_key_prefix,
            },
            auth: AuthConfig {
                jwt_issuer,
                jwt_audience,
                jwt_signing_key,
                jwt_access_token_ttl_seconds,
                jwt_refresh_token_ttl_seconds,
                blacklist_mode,
                google: GoogleAuthConfig {
                    enabled: google_auth_enabled,
                    client_id: google_client_id,
                    client_secret: google_client_secret,
                    redirect_uri: google_redirect_uri,
                    authorization_url: google_authorization_url,
                    token_url: google_token_url,
                    user_info_url: google_user_info_url,
                    http_timeout_seconds: google_http_timeout_seconds,
                    oauth_state_ttl_seconds: google_oauth_state_ttl_seconds,
                },
            },
        })
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.http.host, self.http.port)
    }
}

fn required_var<'a>(vars: &'a HashMap<String, String>, key: &str) -> Result<&'a str> {
    optional_var(vars, key).ok_or_else(|| anyhow!("{key} is required"))
}

fn optional_var<'a>(vars: &'a HashMap<String, String>, key: &str) -> Option<&'a str> {
    vars.get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
}

fn parse_u32(vars: &HashMap<String, String>, key: &str, default_value: u32) -> Result<u32> {
    let value = optional_var(vars, key)
        .map(str::parse::<u32>)
        .transpose()
        .with_context(|| format!("invalid {key} value"))?;

    match value {
        Some(value) if value == 0 => bail!("{key} must be greater than 0"),
        Some(value) => Ok(value),
        None => Ok(default_value),
    }
}

fn parse_u64(vars: &HashMap<String, String>, key: &str, default_value: u64) -> Result<u64> {
    let value = optional_var(vars, key)
        .map(str::parse::<u64>)
        .transpose()
        .with_context(|| format!("invalid {key} value"))?;

    match value {
        Some(value) if value == 0 => bail!("{key} must be greater than 0"),
        Some(value) => Ok(value),
        None => Ok(default_value),
    }
}

fn parse_bool(vars: &HashMap<String, String>, key: &str, default_value: bool) -> Result<bool> {
    optional_var(vars, key)
        .map(str::parse::<bool>)
        .transpose()
        .with_context(|| format!("invalid {key} value"))?
        .map_or(Ok(default_value), Ok)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceConfig {
    pub name: String,
    pub runtime_env: RuntimeEnv,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpConfig {
    pub host: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservabilityConfig {
    pub log_format: LogFormat,
    pub log_filter: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PostgresConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u64,
    pub acquire_timeout_seconds: u64,
    pub run_migrations: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedisConfig {
    pub url: String,
    pub connect_timeout_seconds: u64,
    pub key_prefix: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_signing_key: String,
    pub jwt_access_token_ttl_seconds: u64,
    pub jwt_refresh_token_ttl_seconds: u64,
    pub blacklist_mode: BlacklistMode,
    pub google: GoogleAuthConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleAuthConfig {
    pub enabled: bool,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub authorization_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub http_timeout_seconds: u64,
    pub oauth_state_ttl_seconds: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlacklistMode {
    Disabled,
    Memory,
    Redis,
}

impl FromStr for BlacklistMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "disabled" => Ok(Self::Disabled),
            "memory" => Ok(Self::Memory),
            "redis" => Ok(Self::Redis),
            _ => bail!(
                "unsupported blacklist mode `{value}`; expected `disabled`, `memory`, or `redis`"
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeEnv {
    Local,
    Development,
    Staging,
    Production,
    Test,
}

impl RuntimeEnv {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
            Self::Test => "test",
        }
    }
}

impl fmt::Display for RuntimeEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RuntimeEnv {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "local" => Ok(Self::Local),
            "development" => Ok(Self::Development),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            "test" => Ok(Self::Test),
            _ => bail!(
                "unsupported runtime environment `{value}`; expected one of local, development, staging, production, test"
            ),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

impl FromStr for LogFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            _ => bail!("unsupported log format `{value}`; expected `pretty` or `json`"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{AppConfig, BlacklistMode, LogFormat, RuntimeEnv};

    fn map_from_pairs(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn loads_config_from_env_with_defaults() {
        let vars = map_from_pairs(&[
            ("RUNTIME_ENV", "local"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
        ]);
        let config = AppConfig::from_map(&vars).expect("config should load");

        assert_eq!(config.service.name, "kiro");
        assert_eq!(config.service.runtime_env, RuntimeEnv::Local);
        assert_eq!(config.http.port, 3000);
        assert_eq!(config.observability.log_format, LogFormat::Pretty);
        assert_eq!(config.observability.log_filter, "info");
        assert!(config.postgres.run_migrations);
        assert_eq!(config.redis.connect_timeout_seconds, 3);
        assert_eq!(config.redis.key_prefix, "kiro:local");
        assert_eq!(config.auth.jwt_access_token_ttl_seconds, 7200);
        assert_eq!(config.auth.blacklist_mode, BlacklistMode::Redis);
        assert!(!config.auth.google.enabled);
        assert_eq!(
            config.auth.google.authorization_url,
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(config.auth.google.oauth_state_ttl_seconds, 600);
    }

    #[test]
    fn loads_config_with_explicit_redis_key_prefix() {
        let vars = map_from_pairs(&[
            ("RUNTIME_ENV", "staging"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            ("REDIS_KEY_PREFIX", "shared:tenant-a"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
        ]);
        let config = AppConfig::from_map(&vars).expect("config should load");

        assert_eq!(config.redis.key_prefix, "shared:tenant-a");
    }

    #[test]
    fn fails_when_runtime_env_is_missing() {
        let vars = HashMap::new();
        let error = AppConfig::from_map(&vars).expect_err("config should fail");

        assert!(
            error
                .to_string()
                .contains("missing required runtime environment configuration")
        );
    }

    #[test]
    fn fails_when_port_is_invalid() {
        let vars = map_from_pairs(&[
            ("RUNTIME_ENV", "local"),
            ("HTTP_PORT", "invalid"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
        ]);
        let error = AppConfig::from_map(&vars).expect_err("config should fail");

        assert!(error.to_string().contains("HTTP_PORT"));
    }

    #[test]
    fn fails_when_postgres_url_is_missing() {
        let vars = map_from_pairs(&[
            ("RUNTIME_ENV", "local"),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
        ]);
        let error = AppConfig::from_map(&vars).expect_err("config should fail");

        assert!(
            error
                .to_string()
                .contains("missing required postgres configuration")
        );
    }

    #[test]
    fn fails_when_jwt_signing_key_is_missing() {
        let vars = map_from_pairs(&[
            ("RUNTIME_ENV", "local"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
        ]);
        let error = AppConfig::from_map(&vars).expect_err("config should fail");

        assert!(
            error
                .to_string()
                .contains("missing required jwt signing configuration")
        );
    }

    #[test]
    fn fails_when_google_auth_is_enabled_without_required_credentials() {
        let vars = map_from_pairs(&[
            ("RUNTIME_ENV", "local"),
            (
                "POSTGRES_URL",
                "postgres://postgres:postgres@127.0.0.1:5432/kiro",
            ),
            ("REDIS_URL", "redis://127.0.0.1:6379"),
            (
                "JWT_SIGNING_KEY",
                "test_signing_key_that_is_long_enough_123",
            ),
            ("GOOGLE_AUTH_ENABLED", "true"),
            ("GOOGLE_CLIENT_ID", "google-client-id"),
        ]);
        let error = AppConfig::from_map(&vars).expect_err("config should fail");

        assert!(
            error
                .to_string()
                .contains("GOOGLE_CLIENT_SECRET is required when google auth is enabled")
        );
    }
}
