use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use time::OffsetDateTime;

use crate::domain::repository::user_identity::{
    UserIdentityRepository, UserIdentityRepositoryError,
};
use crate::domain::user_identity::{IdentityProvider, NewUserIdentity, UserIdentity};

#[derive(Clone, Default)]
pub struct InMemoryUserIdentityRepository {
    identities: Arc<Mutex<HashMap<i64, UserIdentity>>>,
}

impl InMemoryUserIdentityRepository {
    #[allow(dead_code)]
    pub fn seeded(identity: UserIdentity) -> Self {
        let mut identities = HashMap::new();
        identities.insert(identity.id, identity);
        Self {
            identities: Arc::new(Mutex::new(identities)),
        }
    }
}

impl UserIdentityRepository for InMemoryUserIdentityRepository {
    async fn create(
        &self,
        new_identity: NewUserIdentity,
    ) -> Result<UserIdentity, UserIdentityRepositoryError> {
        new_identity.validate()?;

        let mut identities = self
            .identities
            .lock()
            .expect("identities mutex should lock");

        if identities.values().any(|identity| {
            identity.provider == new_identity.provider
                && identity.provider_user_id == new_identity.provider_user_id
        }) {
            return Err(UserIdentityRepositoryError::Conflict {
                field: "provider_user_id",
            });
        }

        let identity = UserIdentity {
            id: (identities.len() + 1) as i64,
            identity_code: new_identity.identity_code,
            user_id: new_identity.user_id,
            provider: new_identity.provider,
            provider_user_id: new_identity.provider_user_id,
            provider_email: new_identity.provider_email,
            provider_email_normalized: new_identity.provider_email_normalized,
            profile: new_identity.profile,
            last_authenticated_at: new_identity.last_authenticated_at,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        };

        identities.insert(identity.id, identity.clone());
        Ok(identity)
    }

    async fn find_by_id(
        &self,
        identity_id: i64,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        Ok(self
            .identities
            .lock()
            .expect("identities mutex should lock")
            .get(&identity_id)
            .cloned())
    }

    async fn find_by_identity_code<'a>(
        &'a self,
        identity_code: &'a str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        Ok(self
            .identities
            .lock()
            .expect("identities mutex should lock")
            .values()
            .find(|identity| identity.identity_code == identity_code)
            .cloned())
    }

    async fn find_by_provider_subject<'a>(
        &'a self,
        provider: IdentityProvider,
        provider_user_id: &'a str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        Ok(self
            .identities
            .lock()
            .expect("identities mutex should lock")
            .values()
            .find(|identity| {
                identity.provider == provider && identity.provider_user_id == provider_user_id
            })
            .cloned())
    }

    async fn find_by_provider_email_normalized<'a>(
        &'a self,
        provider: IdentityProvider,
        provider_email_normalized: &'a str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        Ok(self
            .identities
            .lock()
            .expect("identities mutex should lock")
            .values()
            .find(|identity| {
                identity.provider == provider
                    && identity.provider_email_normalized.as_deref()
                        == Some(provider_email_normalized)
            })
            .cloned())
    }

    async fn list_by_user_id(
        &self,
        user_id: i64,
    ) -> Result<Vec<UserIdentity>, UserIdentityRepositoryError> {
        Ok(self
            .identities
            .lock()
            .expect("identities mutex should lock")
            .values()
            .filter(|identity| identity.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn update_last_authenticated_at(
        &self,
        identity_id: i64,
        last_authenticated_at: OffsetDateTime,
    ) -> Result<UserIdentity, UserIdentityRepositoryError> {
        let mut identities = self
            .identities
            .lock()
            .expect("identities mutex should lock");
        let identity = identities
            .get_mut(&identity_id)
            .ok_or(UserIdentityRepositoryError::NotFound)?;
        identity.last_authenticated_at = Some(last_authenticated_at);
        identity.updated_at = last_authenticated_at;
        Ok(identity.clone())
    }
}
