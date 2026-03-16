use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ProtectedSessionResponse {
    pub subject: String,
    pub jti: String,
    pub ua_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub access_token_expires_at: u64,
    pub refresh_token_expires_at: u64,
}

#[derive(Debug, Serialize)]
pub struct LogoutSessionResponse {
    pub subject: String,
    pub access_token_revoked: bool,
    pub refresh_token_revoked: bool,
}
