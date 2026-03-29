pub mod google_login;

use anyhow::Result;
use sqlx::PgPool;

use self::google_login::{GoogleLogin, GoogleLoginLogic};
use crate::domain::{
    entity::user::User,
    repository::{
        user_auth_identity_repository::UserAuthIdentityRepository, user_repository::UserRepository,
    },
};

pub struct AuthLogic<UR, IR> {
    google_login_logic: GoogleLoginLogic<UR, IR>,
}

impl<UR, IR> AuthLogic<UR, IR>
where
    UR: UserRepository,
    IR: UserAuthIdentityRepository,
{
    pub fn new(pool: PgPool, user_repository: UR, user_auth_identity_repository: IR) -> Self {
        Self {
            google_login_logic: GoogleLoginLogic::new(
                pool,
                user_repository,
                user_auth_identity_repository,
            ),
        }
    }

    pub async fn google_login(&self, login: GoogleLogin) -> Result<User> {
        self.google_login_logic.execute(login).await
    }
}
