use std::fmt;
use std::str::FromStr;

use serde_json::Value;
use thiserror::Error;
use time::OffsetDateTime;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewUserIdentity {
    pub identity_code: String,
    pub user_id: i64,
    pub provider: IdentityProvider,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub provider_email_normalized: Option<String>,
    pub profile: Value,
    pub last_authenticated_at: Option<OffsetDateTime>,
}

impl NewUserIdentity {
    pub fn new(
        identity_code: impl Into<String>,
        user_id: i64,
        provider: IdentityProvider,
        provider_user_id: impl Into<String>,
    ) -> Self {
        Self {
            identity_code: identity_code.into(),
            user_id,
            provider,
            provider_user_id: provider_user_id.into(),
            provider_email: None,
            provider_email_normalized: None,
            profile: Value::Object(Default::default()),
            last_authenticated_at: None,
        }
    }

    pub fn with_provider_email(mut self, provider_email: impl AsRef<str>) -> Self {
        let provider_email = normalize_optional_text(provider_email.as_ref());
        self.provider_email_normalized = provider_email
            .as_ref()
            .map(|value| value.to_ascii_lowercase());
        self.provider_email = provider_email;
        self
    }

    pub fn with_profile(mut self, profile: Value) -> Self {
        self.profile = profile;
        self
    }

    pub fn with_last_authenticated_at(mut self, last_authenticated_at: OffsetDateTime) -> Self {
        self.last_authenticated_at = Some(last_authenticated_at);
        self
    }

    pub fn validate(&self) -> Result<(), UserIdentityError> {
        if self.identity_code.trim().is_empty() {
            return Err(UserIdentityError::IdentityCodeRequired);
        }

        if self.user_id <= 0 {
            return Err(UserIdentityError::UserIdInvalid);
        }

        if self.provider_user_id.trim().is_empty() {
            return Err(UserIdentityError::ProviderUserIdRequired);
        }

        if !self.profile.is_object() {
            return Err(UserIdentityError::ProfileMustBeObject);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserIdentity {
    pub id: i64,
    pub identity_code: String,
    pub user_id: i64,
    pub provider: IdentityProvider,
    pub provider_user_id: String,
    pub provider_email: Option<String>,
    pub provider_email_normalized: Option<String>,
    pub profile: Value,
    pub last_authenticated_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl UserIdentity {
    pub fn mark_authenticated(&mut self, authenticated_at: OffsetDateTime) {
        self.last_authenticated_at = Some(authenticated_at);
        self.updated_at = authenticated_at;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityProvider {
    Google,
}

impl IdentityProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Google => "google",
        }
    }
}

impl fmt::Display for IdentityProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for IdentityProvider {
    type Err = UserIdentityError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "google" => Ok(Self::Google),
            _ => Err(UserIdentityError::UnknownProvider(value.to_owned())),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UserIdentityError {
    #[error("identity_code is required")]
    IdentityCodeRequired,
    #[error("user_id must be greater than 0")]
    UserIdInvalid,
    #[error("provider_user_id is required")]
    ProviderUserIdRequired,
    #[error("profile must be a json object")]
    ProfileMustBeObject,
    #[error("unknown identity provider `{0}`")]
    UnknownProvider(String),
}

fn normalize_optional_text(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use time::OffsetDateTime;

    use super::{IdentityProvider, NewUserIdentity, UserIdentity, UserIdentityError};

    #[test]
    fn new_user_identity_normalizes_provider_email() {
        let now = OffsetDateTime::now_utc();
        let identity =
            NewUserIdentity::new("identity_1", 42, IdentityProvider::Google, "google-42")
                .with_provider_email("  Hello.World@example.com  ")
                .with_last_authenticated_at(now);

        assert_eq!(
            identity.provider_email.as_deref(),
            Some("Hello.World@example.com")
        );
        assert_eq!(
            identity.provider_email_normalized.as_deref(),
            Some("hello.world@example.com")
        );
        assert_eq!(identity.last_authenticated_at, Some(now));
    }

    #[test]
    fn new_user_identity_requires_profile_object() {
        let identity =
            NewUserIdentity::new("identity_1", 42, IdentityProvider::Google, "google-42")
                .with_profile(json!(["not-an-object"]));

        assert_eq!(
            identity.validate(),
            Err(UserIdentityError::ProfileMustBeObject)
        );
    }

    #[test]
    fn mark_authenticated_updates_last_authenticated_at() {
        let now = OffsetDateTime::now_utc();
        let mut identity = UserIdentity {
            id: 1,
            identity_code: "identity_1".to_owned(),
            user_id: 7,
            provider: IdentityProvider::Google,
            provider_user_id: "google-7".to_owned(),
            provider_email: None,
            provider_email_normalized: None,
            profile: json!({}),
            last_authenticated_at: None,
            created_at: now,
            updated_at: now,
        };

        let authenticated_at = now + time::Duration::minutes(5);
        identity.mark_authenticated(authenticated_at);

        assert_eq!(identity.last_authenticated_at, Some(authenticated_at));
        assert_eq!(identity.updated_at, authenticated_at);
    }
}
