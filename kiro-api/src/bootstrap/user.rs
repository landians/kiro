use sqlx::PgPool;

use crate::{
    application::user::UserLogic, infrastructure::persistence::user_repository::UserRepository,
};

pub fn build_user_logic(pg_pool: PgPool) -> UserLogic<UserRepository> {
    let user_repository = UserRepository::new(pg_pool);

    UserLogic::new(user_repository)
}
