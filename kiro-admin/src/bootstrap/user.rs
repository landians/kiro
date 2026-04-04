use sqlx::PgPool;

use crate::{
    application::user::AdminUserLogic,
    infrastructure::persistence::admin_user_repository::AdminUserRepository,
};

pub fn build_admin_user_logic(pg_pool: PgPool) -> AdminUserLogic<AdminUserRepository> {
    let admin_user_repository = AdminUserRepository::new(pg_pool);

    AdminUserLogic::new(admin_user_repository)
}
