use crate::infrastructure::auth::blacklist::{TokenBlacklistError, TokenBlacklistService};
use crate::infrastructure::auth::jwt::{
    IssuedToken, JwtError, JwtService, TokenPair, ValidatedToken,
};

#[derive(Clone)]
pub struct AuthService {
    jwt_service: JwtService,
    token_blacklist_service: TokenBlacklistService,
}

impl AuthService {
    pub fn new(jwt_service: JwtService, token_blacklist_service: TokenBlacklistService) -> Self {
        Self {
            jwt_service,
            token_blacklist_service,
        }
    }

    pub fn issue_token_pair(&self, subject: &str, user_agent: &str) -> Result<TokenPair, JwtError> {
        self.jwt_service.issue_token_pair(subject, user_agent)
    }

    pub fn issue_access_token(
        &self,
        subject: &str,
        user_agent: &str,
    ) -> Result<IssuedToken, JwtError> {
        self.jwt_service.issue_access_token(subject, user_agent)
    }

    pub fn issue_refresh_token(
        &self,
        subject: &str,
        user_agent: &str,
    ) -> Result<IssuedToken, JwtError> {
        self.jwt_service.issue_refresh_token(subject, user_agent)
    }

    pub fn validate_access_token(&self, token: &str) -> Result<ValidatedToken, JwtError> {
        self.jwt_service
            .validate_token(token, crate::infrastructure::auth::jwt::TokenKind::Access)
    }

    pub fn validate_refresh_token(&self, token: &str) -> Result<ValidatedToken, JwtError> {
        self.jwt_service
            .validate_token(token, crate::infrastructure::auth::jwt::TokenKind::Refresh)
    }

    pub fn hash_user_agent(&self, user_agent: &str) -> Result<String, JwtError> {
        self.jwt_service.hash_user_agent(user_agent)
    }

    pub async fn is_access_token_revoked(&self, jti: &str) -> Result<bool, TokenBlacklistError> {
        self.token_blacklist_service
            .is_revoked(crate::infrastructure::auth::jwt::TokenKind::Access, jti)
            .await
    }

    pub async fn is_refresh_token_revoked(&self, jti: &str) -> Result<bool, TokenBlacklistError> {
        self.token_blacklist_service
            .is_revoked(crate::infrastructure::auth::jwt::TokenKind::Refresh, jti)
            .await
    }

    pub async fn revoke_access_token(
        &self,
        jti: &str,
        expires_at: u64,
    ) -> Result<(), TokenBlacklistError> {
        self.token_blacklist_service
            .revoke(
                crate::infrastructure::auth::jwt::TokenKind::Access,
                jti,
                expires_at,
            )
            .await
    }

    pub async fn revoke_refresh_token(
        &self,
        jti: &str,
        expires_at: u64,
    ) -> Result<(), TokenBlacklistError> {
        self.token_blacklist_service
            .revoke(
                crate::infrastructure::auth::jwt::TokenKind::Refresh,
                jti,
                expires_at,
            )
            .await
    }

    pub async fn revoke_session_tokens(
        &self,
        access_jti: &str,
        access_expires_at: u64,
        refresh_jti: &str,
        refresh_expires_at: u64,
    ) -> Result<(), TokenBlacklistError> {
        self.revoke_access_token(access_jti, access_expires_at)
            .await?;
        self.revoke_refresh_token(refresh_jti, refresh_expires_at)
            .await
    }
}
