use sqlx::PgPool;

use crate::{
    application::user::{AdminUserLogic, UserLogic},
    infrastructure::persistence::{
        admin_user_repository::AdminUserRepository, user_repository::UserRepository,
    },
};

pub fn build_admin_user_logic(pg_pool: PgPool) -> AdminUserLogic<AdminUserRepository> {
    let admin_user_repository = AdminUserRepository::new(pg_pool);

    AdminUserLogic::new(admin_user_repository)
}

pub fn build_user_logic(pg_pool: PgPool) -> UserLogic<UserRepository> {
    let user_repository = UserRepository::new(pg_pool);

    UserLogic::new(user_repository)
}
