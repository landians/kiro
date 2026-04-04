use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};

use crate::domain::service::admin_password_service::AdminPasswordService;

#[derive(Debug, Clone, Default)]
pub struct PasswordService;

impl PasswordService {
    pub fn new() -> Self {
        Self
    }
}

impl AdminPasswordService for PasswordService {
    fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|error| anyhow!("failed to hash admin password: {error}"))?;

        Ok(password_hash.to_string())
    }

    fn verify_password(&self, password: &str, password_hash: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(password_hash)
            .map_err(|error| anyhow!("failed to parse admin password hash: {error}"))?;
        let verified = Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok();

        Ok(verified)
    }
}

#[cfg(test)]
mod tests {
    use super::PasswordService;
    use crate::domain::service::admin_password_service::AdminPasswordService;

    #[test]
    fn hash_password_produces_argon2_hash() {
        let password_service = PasswordService::new();

        let password_hash = password_service
            .hash_password("admin-password")
            .expect("password hash should be generated");

        assert!(password_hash.starts_with("$argon2"));
    }

    #[test]
    fn verify_password_returns_true_for_matching_password() {
        let password_service = PasswordService::new();
        let password_hash = password_service
            .hash_password("admin-password")
            .expect("password hash should be generated");

        let verified = password_service
            .verify_password("admin-password", &password_hash)
            .expect("password verification should succeed");

        assert!(verified);
    }

    #[test]
    fn verify_password_returns_false_for_non_matching_password() {
        let password_service = PasswordService::new();
        let password_hash = password_service
            .hash_password("admin-password")
            .expect("password hash should be generated");

        let verified = password_service
            .verify_password("another-password", &password_hash)
            .expect("password verification should succeed");

        assert!(!verified);
    }

    #[test]
    fn hash_password_uses_random_salt() {
        let password_service = PasswordService::new();

        let first_hash = password_service
            .hash_password("admin-password")
            .expect("first password hash should be generated");
        let second_hash = password_service
            .hash_password("admin-password")
            .expect("second password hash should be generated");

        assert_ne!(first_hash, second_hash);
    }
}
