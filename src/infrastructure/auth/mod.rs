use std::time::Duration;

use anyhow::Result;
use redis::Client as RedisClient;

use crate::config::AuthConfig;

pub mod blacklist;
pub mod jwt;

#[derive(Clone)]
pub struct AuthInfrastructure {
    pub jwt_service: jwt::JwtService,
    pub blacklist_service: blacklist::TokenBlacklistService,
}

impl AuthInfrastructure {
    pub fn new(
        config: AuthConfig,
        redis_client: Option<RedisClient>,
        redis_timeout_seconds: u64,
        redis_key_prefix: String,
    ) -> Result<Self> {
        let jwt_service = jwt::JwtServiceBuilder::new(config.clone()).build()?;
        let blacklist_service = blacklist::TokenBlacklistServiceBuilder::new(config.blacklist_mode)
            .with_redis_client(redis_client, Duration::from_secs(redis_timeout_seconds))
            .with_key_prefix(redis_key_prefix)
            .build();

        Ok(Self {
            jwt_service,
            blacklist_service,
        })
    }
}
