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
    pub provider_profile: serde_json::Value,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthProvider {
    Google,
}
