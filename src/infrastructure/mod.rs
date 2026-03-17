use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use redis::Client as RedisClient;
use sqlx::PgPool;
use tracing::{error, info, warn};

use crate::config::AppConfig;

pub mod auth;
pub mod persistence;

#[derive(Clone)]
pub struct BootstrapResources {
    pub google_oauth_client: Option<auth::google::GoogleOAuthClient>,
    pub google_oauth_state_service: Option<auth::google_state::GoogleOAuthStateService>,
    #[allow(dead_code)]
    pub postgres_pool: Option<PgPool>,
    #[allow(dead_code)]
    pub redis_client: Option<RedisClient>,
    pub jwt_service: auth::jwt::JwtService,
    pub token_blacklist_service: auth::blacklist::TokenBlacklistService,
    pub readiness: Arc<ReadinessState>,
}

impl BootstrapResources {
    #[cfg(test)]
    pub fn ready_for_test(config: &AppConfig) -> Self {
        Self {
            google_oauth_client: None,
            google_oauth_state_service: None,
            postgres_pool: None,
            redis_client: None,
            jwt_service: auth::jwt::JwtServiceBuilder::new(config.auth.clone())
                .build()
                .expect("test jwt service should build"),
            token_blacklist_service: auth::blacklist::TokenBlacklistServiceBuilder::new(
                config.auth.blacklist_mode,
            )
            .with_redis_client(
                None,
                Duration::from_secs(config.redis.connect_timeout_seconds),
            )
            .with_key_prefix(config.redis.key_prefix.clone())
            .build(),
            readiness: Arc::new(ReadinessState::all_ready()),
        }
    }

    #[cfg(test)]
    pub fn not_ready_for_test(
        config: &AppConfig,
        postgres_reason: &str,
        redis_reason: &str,
    ) -> Self {
        Self {
            google_oauth_client: None,
            google_oauth_state_service: None,
            postgres_pool: None,
            redis_client: None,
            jwt_service: auth::jwt::JwtServiceBuilder::new(config.auth.clone())
                .build()
                .expect("test jwt service should build"),
            token_blacklist_service: auth::blacklist::TokenBlacklistServiceBuilder::new(
                config.auth.blacklist_mode,
            )
            .with_redis_client(
                None,
                Duration::from_secs(config.redis.connect_timeout_seconds),
            )
            .with_key_prefix(config.redis.key_prefix.clone())
            .build(),
            readiness: Arc::new(ReadinessState {
                postgres: DependencyHealth::not_ready(postgres_reason),
                redis: DependencyHealth::not_ready(redis_reason),
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReadinessState {
    pub postgres: DependencyHealth,
    pub redis: DependencyHealth,
}

impl ReadinessState {
    #[cfg(test)]
    pub fn all_ready() -> Self {
        Self {
            postgres: DependencyHealth::ready(),
            redis: DependencyHealth::ready(),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.postgres.is_ready() && self.redis.is_ready()
    }
}

#[derive(Clone, Debug)]
pub enum DependencyHealth {
    Ready,
    NotReady { reason: String },
}

impl DependencyHealth {
    pub fn ready() -> Self {
        Self::Ready
    }

    pub fn not_ready(reason: impl Into<String>) -> Self {
        Self::NotReady {
            reason: reason.into(),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn status_label(&self) -> &'static str {
        match self {
            Self::Ready => "ok",
            Self::NotReady { .. } => "error",
        }
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Ready => None,
            Self::NotReady { reason } => Some(reason.as_str()),
        }
    }
}

pub async fn bootstrap(config: &AppConfig) -> Result<BootstrapResources> {
    let postgres = bootstrap_postgres(config).await;
    let redis = bootstrap_redis(config).await;
    let google_oauth_client =
        auth::google::GoogleOAuthClientBuilder::new(config.auth.google.clone()).build()?;
    let google_oauth_state_service =
        auth::google_state::GoogleOAuthStateServiceBuilder::new(config.auth.clone()).build()?;
    let jwt_service = auth::jwt::JwtServiceBuilder::new(config.auth.clone()).build()?;
    let token_blacklist_service =
        auth::blacklist::TokenBlacklistServiceBuilder::new(config.auth.blacklist_mode)
            .with_redis_client(
                redis.client.clone(),
                Duration::from_secs(config.redis.connect_timeout_seconds),
            )
            .with_key_prefix(config.redis.key_prefix.clone())
            .build();

    let readiness = Arc::new(ReadinessState {
        postgres: postgres.health,
        redis: redis.health,
    });

    if readiness.is_ready() {
        info!("all infrastructure dependencies are ready");
    } else {
        warn!("application started with one or more dependencies not ready");
    }

    Ok(BootstrapResources {
        google_oauth_client,
        google_oauth_state_service,
        postgres_pool: postgres.pool,
        redis_client: redis.client,
        jwt_service,
        token_blacklist_service,
        readiness,
    })
}

struct PostgresBootstrap {
    pool: Option<PgPool>,
    health: DependencyHealth,
}

struct RedisBootstrap {
    client: Option<RedisClient>,
    health: DependencyHealth,
}

async fn bootstrap_postgres(config: &AppConfig) -> PostgresBootstrap {
    let builder = persistence::postgres::PostgresPoolBuilder::new(config.postgres.clone());

    match builder.build().await {
        Ok(pool) => {
            if let Err(error) = persistence::postgres::verify_connectivity(&pool).await {
                error!(error = %error, "postgres connectivity check failed");
                return PostgresBootstrap {
                    pool: Some(pool),
                    health: DependencyHealth::not_ready(error.to_string()),
                };
            }

            if config.postgres.run_migrations {
                if let Err(error) = persistence::postgres::run_migrations(&pool).await {
                    error!(error = %error, "postgres migration execution failed");
                    return PostgresBootstrap {
                        pool: Some(pool),
                        health: DependencyHealth::not_ready(error.to_string()),
                    };
                }
            }

            info!("postgres dependency is ready");
            PostgresBootstrap {
                pool: Some(pool),
                health: DependencyHealth::ready(),
            }
        }
        Err(error) => {
            error!(error = %error, "postgres bootstrap failed");
            PostgresBootstrap {
                pool: None,
                health: DependencyHealth::not_ready(error.to_string()),
            }
        }
    }
}

async fn bootstrap_redis(config: &AppConfig) -> RedisBootstrap {
    let builder = persistence::redis::RedisClientBuilder::new(config.redis.clone());

    match builder.build() {
        Ok(client) => match persistence::redis::verify_connectivity(&client, &config.redis).await {
            Ok(()) => {
                info!("redis dependency is ready");
                RedisBootstrap {
                    client: Some(client),
                    health: DependencyHealth::ready(),
                }
            }
            Err(error) => {
                error!(error = %error, "redis connectivity check failed");
                RedisBootstrap {
                    client: Some(client),
                    health: DependencyHealth::not_ready(error.to_string()),
                }
            }
        },
        Err(error) => {
            error!(error = %error, "redis bootstrap failed");
            RedisBootstrap {
                client: None,
                health: DependencyHealth::not_ready(error.to_string()),
            }
        }
    }
}
