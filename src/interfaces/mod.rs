pub mod controller;
pub mod dto;
pub mod error;
pub mod middleware;

use std::sync::Arc;

use crate::{
    application::auth::AuthLogic,
    infrastructure::{
        auth::{AuthService, GoogleAuthService},
        observability::HttpObservability,
        persistence::{
            user_auth_identity_repository::UserAuthIdentityRepository,
            user_repository::UserRepository,
        },
    },
};

#[derive(Clone)]
pub struct SharedState {
    inner: Arc<SharedStateInner>,
}

struct SharedStateInner {
    auth_service: AuthService,
    google_auth_service: GoogleAuthService,
    http_observability: HttpObservability,
    auth_logic: AuthLogic<UserRepository, UserAuthIdentityRepository>,
}

impl SharedState {
    pub fn new(
        auth_service: AuthService,
        google_auth_service: GoogleAuthService,
        http_observability: HttpObservability,
        auth_logic: AuthLogic<UserRepository, UserAuthIdentityRepository>,
    ) -> Self {
        Self {
            inner: Arc::new(SharedStateInner {
                auth_service,
                google_auth_service,
                http_observability,
                auth_logic,
            }),
        }
    }

    pub fn auth_service(&self) -> &AuthService {
        &self.inner.auth_service
    }

    pub fn google_auth_service(&self) -> &GoogleAuthService {
        &self.inner.google_auth_service
    }

    pub fn http_observability(&self) -> &HttpObservability {
        &self.inner.http_observability
    }

    pub fn auth_logic(&self) -> &AuthLogic<UserRepository, UserAuthIdentityRepository> {
        &self.inner.auth_logic
    }
}
