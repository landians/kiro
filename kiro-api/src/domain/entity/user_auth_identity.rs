#![allow(dead_code)]

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAuthIdentity {
    pub id: i64,
    pub user_id: i64,
    pub provider: AuthProvider,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub provider_email_verified: bool,
    pub provider_display_name: Option<String>,
    pub provider_avatar_url: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthProvider {
    Google,
}

impl AuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Google => "google",
        }
    }

    pub fn from_db(value: &str) -> Result<Self> {
        match value {
            "google" => Ok(Self::Google),
            other => Err(anyhow!("unsupported auth provider: {other}")),
        }
    }
}
