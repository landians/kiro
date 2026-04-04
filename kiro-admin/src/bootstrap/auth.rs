use sqlx::PgPool;

use crate::{
    application::auth::AuthLogic,
    infrastructure::{
        auth::password::PasswordService, persistence::admin_user_repository::AdminUserRepository,
    },
};

pub fn build_auth_logic(pg_pool: PgPool) -> AuthLogic<AdminUserRepository, PasswordService> {
    let admin_user_repository = AdminUserRepository::new(pg_pool);
    let password_service = PasswordService::new();

    AuthLogic::new(admin_user_repository, password_service)
}
