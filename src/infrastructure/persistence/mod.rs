use anyhow::{Ok, Result};
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::infrastructure::config::PgConfig;

pub struct PostgresBuilder {
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: u16,
    pub database: String,
}

impl PostgresBuilder {
    pub fn new(c: PgConfig) -> PostgresBuilder {
        PostgresBuilder {
            username: c.username,
            password: c.password,
            host: c.host,
            port: c.port,
            database: c.database,
        }
    }

    pub async fn build(self) -> Result<PgPool> {
        let dsn = format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        );

        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&dsn)
            .await?;

        Ok(pool)
    }
}
