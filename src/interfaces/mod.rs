pub mod controller;
pub mod dto;
pub mod error;
pub mod middleware;

use std::sync::Arc;

use crate::{
    application::auth::AppAuthLogic,
    infrastructure::auth::{AuthService, GoogleAuthService},
};

#[derive(Clone)]
pub struct SharedState {
    inner: Arc<SharedStateInner>,
}

struct SharedStateInner {
    auth_service: AuthService,
    google_auth_service: GoogleAuthService,
    auth_logic: AppAuthLogic,
}

impl SharedState {
    pub fn new(
        auth_service: AuthService,
        google_auth_service: GoogleAuthService,
        auth_logic: AppAuthLogic,
    ) -> Self {
        Self {
            inner: Arc::new(SharedStateInner {
                auth_service,
                google_auth_service,
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

    pub fn auth_logic(&self) -> &AppAuthLogic {
        &self.inner.auth_logic
    }
}
