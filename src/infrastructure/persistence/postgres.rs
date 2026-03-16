use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::config::PostgresConfig;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub struct PostgresPoolBuilder {
    config: PostgresConfig,
}

impl PostgresPoolBuilder {
    pub fn new(config: PostgresConfig) -> Self {
        Self { config }
    }

    pub async fn build(self) -> Result<PgPool> {
        let connect_future = PgPoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(Duration::from_secs(self.config.acquire_timeout_seconds))
            .connect(&self.config.url);

        tokio::time::timeout(
            Duration::from_secs(self.config.connect_timeout_seconds),
            connect_future,
        )
        .await
        .context("timed out while creating postgres connection pool")?
        .context("failed to create postgres connection pool")
    }
}

pub async fn verify_connectivity(pool: &PgPool) -> Result<()> {
    sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(pool)
        .await
        .context("failed to verify postgres connectivity")?;

    Ok(())
}

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    MIGRATOR
        .run(pool)
        .await
        .context("failed to run postgres migrations")
}
