use std::fmt;
use std::str::FromStr;
use thiserror::Error;
use time::OffsetDateTime;

const DEFAULT_LOCALE: &str = "en-US";
const DEFAULT_TIME_ZONE: &str = "UTC";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewUser {
    pub user_code: String,
    pub email: Option<String>,
    pub email_normalized: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub locale: String,
    pub time_zone: String,
    pub status: UserStatus,
}

impl NewUser {
    pub fn new(user_code: impl Into<String>) -> Self {
        Self {
            user_code: user_code.into(),
            email: None,
            email_normalized: None,
            display_name: None,
            avatar_url: None,
            locale: DEFAULT_LOCALE.to_owned(),
            time_zone: DEFAULT_TIME_ZONE.to_owned(),
            status: UserStatus::Pending,
        }
    }

    pub fn with_email(mut self, email: impl AsRef<str>) -> Self {
        let email = normalize_optional_text(email.as_ref());
        self.email_normalized = email.as_ref().map(|value| value.to_ascii_lowercase());
        self.email = email;
        self
    }

    pub fn with_display_name(mut self, display_name: impl AsRef<str>) -> Self {
        self.display_name = normalize_optional_text(display_name.as_ref());
        self
    }

    pub fn with_avatar_url(mut self, avatar_url: impl AsRef<str>) -> Self {
        self.avatar_url = normalize_optional_text(avatar_url.as_ref());
        self
    }

    pub fn with_locale(mut self, locale: impl AsRef<str>) -> Self {
        self.locale = locale.as_ref().trim().to_owned();
        self
    }

    pub fn with_time_zone(mut self, time_zone: impl AsRef<str>) -> Self {
        self.time_zone = time_zone.as_ref().trim().to_owned();
        self
    }

    pub fn with_status(mut self, status: UserStatus) -> Self {
        self.status = status;
        self
    }

    pub fn validate(&self) -> Result<(), UserError> {
        if self.user_code.trim().is_empty() {
            return Err(UserError::UserCodeRequired);
        }

        if self.locale.trim().is_empty() {
            return Err(UserError::LocaleRequired);
        }

        if self.time_zone.trim().is_empty() {
            return Err(UserError::TimeZoneRequired);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub user_code: String,
    pub email: Option<String>,
    pub email_normalized: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub locale: String,
    pub time_zone: String,
    pub status: UserStatus,
    pub last_login_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

impl User {
    pub fn activate(&mut self) -> Result<(), UserError> {
        self.transition_to(UserStatus::Active)
    }

    pub fn disable(&mut self) -> Result<(), UserError> {
        self.transition_to(UserStatus::Disabled)
    }

    pub fn restore(&mut self) -> Result<(), UserError> {
        self.transition_to(UserStatus::Active)
    }

    pub fn mark_deleted(&mut self) -> Result<(), UserError> {
        self.transition_to(UserStatus::Deleted)
    }

    pub fn transition_to(&mut self, next_status: UserStatus) -> Result<(), UserError> {
        if self.status == next_status {
            return Ok(());
        }

        if !self.status.can_transition_to(next_status) {
            return Err(UserError::InvalidStatusTransition {
                from: self.status,
                to: next_status,
            });
        }

        self.status = next_status;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UserStatus {
    Pending,
    Active,
    Disabled,
    Deleted,
}

impl UserStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Disabled => "disabled",
            Self::Deleted => "deleted",
        }
    }

    pub fn can_transition_to(self, next_status: UserStatus) -> bool {
        match (self, next_status) {
            (Self::Pending, Self::Active | Self::Deleted) => true,
            (Self::Active, Self::Disabled | Self::Deleted) => true,
            (Self::Disabled, Self::Active | Self::Deleted) => true,
            _ => false,
        }
    }
}

impl fmt::Display for UserStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for UserStatus {
    type Err = UserError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(Self::Pending),
            "active" => Ok(Self::Active),
            "disabled" => Ok(Self::Disabled),
            "deleted" => Ok(Self::Deleted),
            _ => Err(UserError::UnknownStatus(value.to_owned())),
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum UserError {
    #[error("user_code is required")]
    UserCodeRequired,
    #[error("locale is required")]
    LocaleRequired,
    #[error("time_zone is required")]
    TimeZoneRequired,
    #[error("cannot transition user status from `{from}` to `{to}`")]
    InvalidStatusTransition { from: UserStatus, to: UserStatus },
    #[error("unknown user status `{0}`")]
    UnknownStatus(String),
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
    use time::OffsetDateTime;

    use super::{NewUser, User, UserError, UserStatus};

    #[test]
    fn new_user_normalizes_email_and_defaults_locale() {
        let user = NewUser::new("user_1")
            .with_email("  Hello.World@example.com  ")
            .with_display_name("  Hello  ")
            .with_avatar_url(" https://example.com/avatar.png ")
            .with_locale(" zh-CN ")
            .with_time_zone(" Asia/Shanghai ");

        assert_eq!(user.email.as_deref(), Some("Hello.World@example.com"));
        assert_eq!(
            user.email_normalized.as_deref(),
            Some("hello.world@example.com")
        );
        assert_eq!(user.display_name.as_deref(), Some("Hello"));
        assert_eq!(
            user.avatar_url.as_deref(),
            Some("https://example.com/avatar.png")
        );
        assert_eq!(user.locale, "zh-CN");
        assert_eq!(user.time_zone, "Asia/Shanghai");
        assert_eq!(user.status, UserStatus::Pending);
    }

    #[test]
    fn new_user_rejects_missing_user_code() {
        let user = NewUser::new("   ");

        assert_eq!(user.validate(), Err(UserError::UserCodeRequired));
    }

    #[test]
    fn user_status_allows_expected_transitions() {
        let now = OffsetDateTime::now_utc();
        let mut user = User {
            id: 1,
            user_code: "user_1".to_owned(),
            email: None,
            email_normalized: None,
            display_name: None,
            avatar_url: None,
            locale: "en-US".to_owned(),
            time_zone: "UTC".to_owned(),
            status: UserStatus::Pending,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        };

        user.activate().expect("pending -> active should succeed");
        assert_eq!(user.status, UserStatus::Active);

        user.disable().expect("active -> disabled should succeed");
        assert_eq!(user.status, UserStatus::Disabled);

        user.restore().expect("disabled -> active should succeed");
        assert_eq!(user.status, UserStatus::Active);

        user.mark_deleted()
            .expect("active -> deleted should succeed");
        assert_eq!(user.status, UserStatus::Deleted);
    }

    #[test]
    fn user_status_rejects_invalid_transition() {
        let now = OffsetDateTime::now_utc();
        let mut user = User {
            id: 1,
            user_code: "user_1".to_owned(),
            email: None,
            email_normalized: None,
            display_name: None,
            avatar_url: None,
            locale: "en-US".to_owned(),
            time_zone: "UTC".to_owned(),
            status: UserStatus::Deleted,
            last_login_at: None,
            created_at: now,
            updated_at: now,
        };

        let error = user
            .restore()
            .expect_err("deleted -> active should be rejected");

        assert_eq!(
            error,
            UserError::InvalidStatusTransition {
                from: UserStatus::Deleted,
                to: UserStatus::Active,
            }
        );
    }
}
