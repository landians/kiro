pub mod controller;
pub mod dto;
pub mod error;
pub mod middleware;

use std::sync::Arc;

use crate::{
    application::{auth::AuthLogic, user::AdminUserLogic},
    infrastructure::{
        auth::{AuthService, password::PasswordService},
        observability::HttpObservability,
        persistence::admin_user_repository::AdminUserRepository,
    },
};

#[derive(Clone)]
pub struct SharedState {
    inner: Arc<SharedStateInner>,
}

struct SharedStateInner {
    auth_service: AuthService,
    http_observability: HttpObservability,
    auth_logic: AuthLogic<AdminUserRepository, PasswordService>,
    admin_user_logic: AdminUserLogic<AdminUserRepository>,
}

impl SharedState {
    pub fn new(
        auth_service: AuthService,
        http_observability: HttpObservability,
        auth_logic: AuthLogic<AdminUserRepository, PasswordService>,
        admin_user_logic: AdminUserLogic<AdminUserRepository>,
    ) -> Self {
        Self {
            inner: Arc::new(SharedStateInner {
                auth_service,
                http_observability,
                auth_logic,
                admin_user_logic,
            }),
        }
    }

    pub fn auth_service(&self) -> &AuthService {
        &self.inner.auth_service
    }

    pub fn http_observability(&self) -> &HttpObservability {
        &self.inner.http_observability
    }

    pub fn auth_logic(&self) -> &AuthLogic<AdminUserRepository, PasswordService> {
        &self.inner.auth_logic
    }

    pub fn admin_user_logic(&self) -> &AdminUserLogic<AdminUserRepository> {
        &self.inner.admin_user_logic
    }
}
