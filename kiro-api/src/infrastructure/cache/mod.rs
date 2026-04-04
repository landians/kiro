use anyhow::{Ok, Result};
use redis::{Client, aio::MultiplexedConnection};

use crate::infrastructure::config::RedisConfig;

pub struct CacheBuilder {
    pub password: Option<String>,
    pub host: String,
    pub port: u16,
}

impl CacheBuilder {
    pub fn new(c: RedisConfig) -> CacheBuilder {
        CacheBuilder {
            password: c.password,
            host: c.host,
            port: c.port,
        }
    }

    #[tracing::instrument(
        skip(self),
        fields(cache.system = "redis", cache.host = %self.host, cache.port = self.port)
    )]
    pub async fn build(self) -> Result<MultiplexedConnection> {
        let dsn = if let Some(pwd) = self.password {
            format!("redis://:{}@{}:{}", pwd, self.host, self.port)
        } else {
            format!("redis://{}:{}", self.host, self.port)
        };

        let conn = Client::open(dsn)?
            .get_multiplexed_async_connection()
            .await?;

        Ok(conn)
    }
}
