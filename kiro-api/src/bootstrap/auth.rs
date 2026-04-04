use sqlx::PgPool;

use crate::{
    application::auth::AuthLogic,
    infrastructure::persistence::{
        user_auth_identity_repository::UserAuthIdentityRepository, user_repository::UserRepository,
    },
};

pub fn build_auth_logic(pg_pool: PgPool) -> AuthLogic<UserRepository, UserAuthIdentityRepository> {
    let user_repository = UserRepository::new(pg_pool.clone());
    let user_auth_identity_repository = UserAuthIdentityRepository::new(pg_pool.clone());

    AuthLogic::new(pg_pool, user_repository, user_auth_identity_repository)
}
