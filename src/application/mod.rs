use crate::application::auth::AuthService;
use crate::application::health::HealthService;

pub mod auth;
pub mod health;

#[derive(Clone)]
pub struct AppServices {
    pub auth: AuthService,
    pub health: HealthService,
}

impl AppServices {
    pub fn new(auth: AuthService, health: HealthService) -> Self {
        Self { auth, health }
    }
}
