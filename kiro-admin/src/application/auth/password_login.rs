use anyhow::Result;
use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::domain::{
    entity::admin_user::AdminUser, repository::admin_user_repository::AdminUserRepository,
    service::admin_password_service::AdminPasswordService,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PasswordLogin {
    pub email: String,
    pub password: String,
    pub login_at: DateTime<Utc>,
}

pub struct PasswordLoginLogic<AR, PS> {
    admin_user_repository: AR,
    password_service: PS,
}

impl<AR, PS> PasswordLoginLogic<AR, PS>
where
    AR: AdminUserRepository,
    PS: AdminPasswordService,
{
    pub fn new(admin_user_repository: AR, password_service: PS) -> Self {
        Self {
            admin_user_repository,
            password_service,
        }
    }

    #[tracing::instrument(skip(self, login), fields(admin_user.email = %login.email))]
    pub async fn execute(&self, login: PasswordLogin) -> Result<AdminUser> {
        let Some(admin_user) = self
            .admin_user_repository
            .find_by_email(&login.email)
            .await?
        else {
            return Err(PasswordLoginError::InvalidCredentials.into());
        };

        self.ensure_admin_user_can_login(&admin_user)?;

        let password_matched = self
            .password_service
            .verify_password(&login.password, &admin_user.password_hash)?;
        if !password_matched {
            return Err(PasswordLoginError::InvalidCredentials.into());
        }

        self.admin_user_repository
            .touch_last_login(admin_user.id, login.login_at)
            .await?;

        let mut admin_user = admin_user;
        admin_user.last_login_at = Some(login.login_at);

        Ok(admin_user)
    }

    fn ensure_admin_user_can_login(&self, admin_user: &AdminUser) -> Result<()> {
        if admin_user.is_active() {
            return Ok(());
        }

        if admin_user.is_frozen() {
            return Err(PasswordLoginError::AdminUserFrozen {
                admin_user_id: admin_user.id,
            }
            .into());
        }

        Err(PasswordLoginError::AdminUserFrozen {
            admin_user_id: admin_user.id,
        }
        .into())
    }
}

#[derive(Debug, Error)]
pub enum PasswordLoginError {
    #[error("invalid admin credentials")]
    InvalidCredentials,
    #[error("admin user {admin_user_id} is frozen")]
    AdminUserFrozen { admin_user_id: i64 },
}
