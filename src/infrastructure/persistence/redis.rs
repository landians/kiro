use std::time::Duration;

use anyhow::{Context, Result, bail};
use redis::Client;

use crate::config::RedisConfig;

pub struct RedisClientBuilder {
    config: RedisConfig,
}

impl RedisClientBuilder {
    pub fn new(config: RedisConfig) -> Self {
        Self { config }
    }

    pub fn build(self) -> Result<Client> {
        Client::open(self.config.url.as_str()).context("failed to create redis client")
    }
}

pub async fn verify_connectivity(client: &Client, config: &RedisConfig) -> Result<()> {
    let timeout = Duration::from_secs(config.connect_timeout_seconds);
    let mut connection = tokio::time::timeout(timeout, client.get_multiplexed_async_connection())
        .await
        .context("timed out while opening redis connection")?
        .context("failed to open redis connection")?;

    let pong = tokio::time::timeout(
        timeout,
        redis::cmd("PING").query_async::<String>(&mut connection),
    )
    .await
    .context("timed out while pinging redis")?
    .context("failed to ping redis")?;

    if pong != "PONG" {
        bail!("unexpected redis ping response `{pong}`");
    }

    Ok(())
}
