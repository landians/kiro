use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use redis::Client;
use thiserror::Error;

use crate::config::BlacklistMode;
use crate::infrastructure::auth::jwt::TokenKind;

const BLACKLIST_KEY_SUFFIX: &str = "auth:blacklist";

#[derive(Clone)]
pub struct TokenBlacklistService {
    backend: BlacklistBackend,
    key_namespace: String,
}

impl TokenBlacklistService {
    pub async fn is_revoked(
        &self,
        token_kind: TokenKind,
        jti: &str,
    ) -> Result<bool, TokenBlacklistError> {
        let key = blacklist_key(&self.key_namespace, token_kind, jti);

        match &self.backend {
            BlacklistBackend::Disabled => Ok(false),
            BlacklistBackend::Memory { entries } => Ok(entries
                .lock()
                .expect("blacklist mutex should not be poisoned")
                .contains(&key)),
            BlacklistBackend::Redis {
                client,
                command_timeout,
            } => {
                let mut connection =
                    open_redis_connection(client.as_ref(), *command_timeout).await?;
                let is_revoked = tokio::time::timeout(
                    *command_timeout,
                    redis::cmd("EXISTS")
                        .arg(&key)
                        .query_async::<u8>(&mut connection),
                )
                .await
                .map_err(|_| TokenBlacklistError::RedisCommandTimeout)?
                .map_err(TokenBlacklistError::RedisCommandFailed)?;

                Ok(is_revoked > 0)
            }
        }
    }

    pub async fn revoke(
        &self,
        token_kind: TokenKind,
        jti: &str,
        expires_at: u64,
    ) -> Result<(), TokenBlacklistError> {
        self.revoke_many(&[TokenRevocation {
            token_kind,
            jti,
            expires_at,
        }])
        .await
    }

    pub async fn revoke_many(
        &self,
        revocations: &[TokenRevocation<'_>],
    ) -> Result<(), TokenBlacklistError> {
        if revocations.is_empty() {
            return Ok(());
        }

        match &self.backend {
            BlacklistBackend::Disabled => Ok(()),
            BlacklistBackend::Memory { entries } => {
                let mut blacklist = entries
                    .lock()
                    .expect("blacklist mutex should not be poisoned");
                for revocation in revocations {
                    blacklist.insert(blacklist_key(
                        &self.key_namespace,
                        revocation.token_kind,
                        revocation.jti,
                    ));
                }
                Ok(())
            }
            BlacklistBackend::Redis {
                client,
                command_timeout,
            } => {
                let mut connection =
                    open_redis_connection(client.as_ref(), *command_timeout).await?;
                let mut pipeline = redis::pipe();
                pipeline.atomic();

                for revocation in revocations {
                    pipeline
                        .cmd("SET")
                        .arg(blacklist_key(
                            &self.key_namespace,
                            revocation.token_kind,
                            revocation.jti,
                        ))
                        .arg("1")
                        .arg("EXAT")
                        .arg(revocation.expires_at)
                        .ignore();
                }

                tokio::time::timeout(
                    *command_timeout,
                    pipeline.query_async::<()>(&mut connection),
                )
                .await
                .map_err(|_| TokenBlacklistError::RedisCommandTimeout)?
                .map_err(TokenBlacklistError::RedisCommandFailed)?;

                Ok(())
            }
        }
    }
}

pub struct TokenBlacklistServiceBuilder {
    mode: BlacklistMode,
    redis_client: Option<Client>,
    redis_command_timeout: Duration,
    redis_key_prefix: String,
}

impl TokenBlacklistServiceBuilder {
    pub fn new(mode: BlacklistMode) -> Self {
        Self {
            mode,
            redis_client: None,
            redis_command_timeout: Duration::from_secs(3),
            redis_key_prefix: String::new(),
        }
    }

    pub fn with_redis_client(
        mut self,
        redis_client: Option<Client>,
        redis_command_timeout: Duration,
    ) -> Self {
        self.redis_client = redis_client;
        self.redis_command_timeout = redis_command_timeout;
        self
    }

    pub fn with_key_prefix(mut self, redis_key_prefix: impl Into<String>) -> Self {
        self.redis_key_prefix = redis_key_prefix.into();
        self
    }

    pub fn build(self) -> TokenBlacklistService {
        let key_namespace = blacklist_namespace(&self.redis_key_prefix);
        let backend = match self.mode {
            BlacklistMode::Disabled => BlacklistBackend::Disabled,
            BlacklistMode::Memory => BlacklistBackend::Memory {
                entries: Arc::new(Mutex::new(HashSet::new())),
            },
            BlacklistMode::Redis => BlacklistBackend::Redis {
                client: self.redis_client,
                command_timeout: self.redis_command_timeout,
            },
        };

        TokenBlacklistService {
            backend,
            key_namespace,
        }
    }
}

#[derive(Clone)]
enum BlacklistBackend {
    Disabled,
    Memory {
        entries: Arc<Mutex<HashSet<String>>>,
    },
    Redis {
        client: Option<Client>,
        command_timeout: Duration,
    },
}

#[derive(Clone, Copy)]
pub struct TokenRevocation<'a> {
    pub token_kind: TokenKind,
    pub jti: &'a str,
    pub expires_at: u64,
}

#[derive(Debug, Error)]
pub enum TokenBlacklistError {
    #[error("redis blacklist backend is unavailable")]
    RedisBackendUnavailable,
    #[error("timed out while opening redis blacklist connection")]
    RedisConnectionTimeout,
    #[error("failed to open redis blacklist connection")]
    RedisConnectionFailed(#[source] redis::RedisError),
    #[error("timed out while executing redis blacklist command")]
    RedisCommandTimeout,
    #[error("failed to execute redis blacklist command")]
    RedisCommandFailed(#[source] redis::RedisError),
}

fn blacklist_key(key_namespace: &str, token_kind: TokenKind, jti: &str) -> String {
    format!("{key_namespace}:{}:{jti}", token_kind.as_str())
}

fn blacklist_namespace(redis_key_prefix: &str) -> String {
    let trimmed = redis_key_prefix.trim_matches(':');
    if trimmed.is_empty() {
        BLACKLIST_KEY_SUFFIX.to_owned()
    } else {
        format!("{trimmed}:{BLACKLIST_KEY_SUFFIX}")
    }
}

async fn open_redis_connection(
    client: Option<&Client>,
    command_timeout: Duration,
) -> Result<redis::aio::MultiplexedConnection, TokenBlacklistError> {
    let client = client.ok_or(TokenBlacklistError::RedisBackendUnavailable)?;

    tokio::time::timeout(command_timeout, client.get_multiplexed_async_connection())
        .await
        .map_err(|_| TokenBlacklistError::RedisConnectionTimeout)?
        .map_err(TokenBlacklistError::RedisConnectionFailed)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::config::BlacklistMode;

    use super::{
        TokenBlacklistError, TokenBlacklistServiceBuilder, TokenKind, blacklist_key,
        blacklist_namespace,
    };

    #[tokio::test]
    async fn memory_blacklist_marks_token_as_revoked() {
        let service = TokenBlacklistServiceBuilder::new(BlacklistMode::Memory).build();

        assert!(
            !service
                .is_revoked(TokenKind::Access, "access_jti_1")
                .await
                .expect("memory blacklist lookup should succeed")
        );

        service
            .revoke(TokenKind::Access, "access_jti_1", u64::MAX)
            .await
            .expect("memory blacklist revoke should succeed");

        assert!(
            service
                .is_revoked(TokenKind::Access, "access_jti_1")
                .await
                .expect("memory blacklist lookup should succeed")
        );
    }

    #[tokio::test]
    async fn redis_blacklist_without_client_returns_backend_unavailable() {
        let service = TokenBlacklistServiceBuilder::new(BlacklistMode::Redis)
            .with_redis_client(None, Duration::from_secs(1))
            .build();

        let error = service
            .is_revoked(TokenKind::Access, "access_jti_1")
            .await
            .expect_err("redis blacklist lookup should fail without client");

        assert!(matches!(
            error,
            TokenBlacklistError::RedisBackendUnavailable
        ));
    }

    #[test]
    fn blacklist_namespace_uses_redis_key_prefix() {
        assert_eq!(
            blacklist_namespace("kiro:staging"),
            "kiro:staging:auth:blacklist"
        );
        assert_eq!(blacklist_namespace("tenant-a:"), "tenant-a:auth:blacklist");
        assert_eq!(blacklist_namespace(""), "auth:blacklist");
    }

    #[test]
    fn blacklist_key_includes_prefix_namespace() {
        let namespace = blacklist_namespace("kiro:prod");

        assert_eq!(
            blacklist_key(&namespace, TokenKind::Refresh, "jti_123"),
            "kiro:prod:auth:blacklist:refresh:jti_123"
        );
    }
}
