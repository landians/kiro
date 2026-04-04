pub mod password_login;

use anyhow::Result;

use self::password_login::{PasswordLogin, PasswordLoginLogic};
use crate::domain::{
    entity::admin_user::AdminUser, repository::admin_user_repository::AdminUserRepository,
    service::admin_password_service::AdminPasswordService,
};

pub struct AuthLogic<AR, PS> {
    password_login_logic: PasswordLoginLogic<AR, PS>,
}

impl<AR, PS> AuthLogic<AR, PS>
where
    AR: AdminUserRepository,
    PS: AdminPasswordService,
{
    pub fn new(admin_user_repository: AR, password_service: PS) -> Self {
        Self {
            password_login_logic: PasswordLoginLogic::new(admin_user_repository, password_service),
        }
    }

    #[tracing::instrument(skip(self, login))]
    pub async fn password_login(&self, login: PasswordLogin) -> Result<AdminUser> {
        self.password_login_logic.execute(login).await
    }
}
