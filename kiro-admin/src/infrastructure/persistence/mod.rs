pub mod admin_user_repository;
pub mod product_repository;
pub mod user_repository;

use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::infrastructure::config::PgConfig;

pub struct PostgresBuilder {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
}

impl PostgresBuilder {
    pub fn new(c: PgConfig) -> PostgresBuilder {
        PostgresBuilder {
            username: c.username,
            password: c.password,
            host: c.host,
            port: c.port,
            database: c.database,
            max_connections: c.max_connections.unwrap_or(20),
            min_connections: c.min_connections.unwrap_or(5),
            acquire_timeout_seconds: c.acquire_timeout_seconds.unwrap_or(5),
            idle_timeout_seconds: c.idle_timeout_seconds.unwrap_or(600),
            max_lifetime_seconds: c.max_lifetime_seconds.unwrap_or(1800),
        }
    }

    #[tracing::instrument(
        skip(self),
        fields(
            db.system = "postgresql",
            db.host = %self.host,
            db.port = self.port,
            db.name = %self.database,
            db.pool.max_connections = self.max_connections,
            db.pool.min_connections = self.min_connections
        )
    )]
    pub async fn build(self) -> Result<PgPool> {
        let dsn = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        );

        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections)
            .min_connections(self.min_connections)
            .acquire_timeout(std::time::Duration::from_secs(self.acquire_timeout_seconds))
            .idle_timeout(Some(std::time::Duration::from_secs(
                self.idle_timeout_seconds,
            )))
            .max_lifetime(Some(std::time::Duration::from_secs(
                self.max_lifetime_seconds,
            )))
            .connect(&dsn)
            .await?;
        Ok(pool)
    }
}
