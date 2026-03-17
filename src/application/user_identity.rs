use time::OffsetDateTime;

use crate::domain::repository::user_identity::{
    UserIdentityRepository, UserIdentityRepositoryError,
};
use crate::domain::user_identity::{IdentityProvider, NewUserIdentity, UserIdentity};
use crate::infrastructure::persistence::postgres::accounts::user_identity_repository::PostgresUserIdentityRepository;

pub type DefaultUserIdentityService = UserIdentityService<PostgresUserIdentityRepository>;

#[derive(Clone)]
pub struct UserIdentityService<R>
where
    R: UserIdentityRepository,
{
    user_identity_repository: R,
}

impl<R> UserIdentityService<R>
where
    R: UserIdentityRepository,
{
    pub fn new(user_identity_repository: R) -> Self {
        Self {
            user_identity_repository,
        }
    }

    pub async fn create_identity(
        &self,
        new_identity: NewUserIdentity,
    ) -> Result<UserIdentity, UserIdentityRepositoryError> {
        self.user_identity_repository.create(new_identity).await
    }

    pub async fn find_identity_by_id(
        &self,
        identity_id: i64,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        self.user_identity_repository.find_by_id(identity_id).await
    }

    pub async fn find_identity_by_code(
        &self,
        identity_code: &str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        self.user_identity_repository
            .find_by_identity_code(identity_code)
            .await
    }

    pub async fn find_identity_by_provider_subject(
        &self,
        provider: IdentityProvider,
        provider_user_id: &str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        self.user_identity_repository
            .find_by_provider_subject(provider, provider_user_id)
            .await
    }

    pub async fn find_identity_by_provider_email(
        &self,
        provider: IdentityProvider,
        provider_email: &str,
    ) -> Result<Option<UserIdentity>, UserIdentityRepositoryError> {
        let Some(provider_email_normalized) = normalize_email(provider_email) else {
            return Ok(None);
        };

        self.user_identity_repository
            .find_by_provider_email_normalized(provider, &provider_email_normalized)
            .await
    }

    pub async fn list_identities_by_user_id(
        &self,
        user_id: i64,
    ) -> Result<Vec<UserIdentity>, UserIdentityRepositoryError> {
        self.user_identity_repository.list_by_user_id(user_id).await
    }

    pub async fn record_authentication(
        &self,
        identity_id: i64,
    ) -> Result<UserIdentity, UserIdentityRepositoryError> {
        self.user_identity_repository
            .update_last_authenticated_at(identity_id, OffsetDateTime::now_utc())
            .await
    }
}

fn normalize_email(email: &str) -> Option<String> {
    let trimmed = email.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use time::OffsetDateTime;

    use super::UserIdentityService;
    use crate::domain::repository::user_identity::{
        UserIdentityRepository, UserIdentityRepositoryError,
    };
    use crate::domain::user_identity::{IdentityProvider, NewUserIdentity, UserIdentity};

    #[derive(Clone, Default)]
    struct TestUserIdentityRepository {
        identities: Arc<Mutex<HashMap<i64, UserIdentity>>>,
    }

    impl TestUserIdentityRepository {
        fn seeded(identity: UserIdentity) -> Self {
            let mut identities = HashMap::new();
            identities.insert(identity.id, identity);
            let identities = Arc::new(Mutex::new(identities));
            Self { identities }
        }
    }

    impl UserIdentityRepository for TestUserIdentityRepository {
        async fn create(
            &self,
            new_identity: NewUserIdentity,
        ) -> Result<UserIdentity, UserIdentityRepositoryError> {
            new_identity.validate()?;

            let mut identities = self
                .identities
                .lock()
                .expect("identities mutex should lock");
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

    fn test_identity() -> UserIdentity {
        let now = OffsetDateTime::now_utc();
        UserIdentity {
            id: 5,
            identity_code: "identity_5".to_owned(),
            user_id: 7,
            provider: IdentityProvider::Google,
            provider_user_id: "google-7".to_owned(),
            provider_email: Some("hello@example.com".to_owned()),
            provider_email_normalized: Some("hello@example.com".to_owned()),
            profile: json!({"sub": "google-7"}),
            last_authenticated_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_find_and_list_identity_via_service() {
        let service = UserIdentityService::new(TestUserIdentityRepository::default());
        let new_identity =
            NewUserIdentity::new("identity_new", 7, IdentityProvider::Google, "google-new")
                .with_provider_email("new@example.com");

        let created = service
            .create_identity(new_identity)
            .await
            .expect("identity create should succeed");

        let found_by_id = service
            .find_identity_by_id(created.id)
            .await
            .expect("find by id should succeed")
            .expect("identity should exist");
        assert_eq!(found_by_id.identity_code, "identity_new");

        let found_by_code = service
            .find_identity_by_code("identity_new")
            .await
            .expect("find by code should succeed")
            .expect("identity should exist");
        assert_eq!(found_by_code.id, created.id);

        let listed = service
            .list_identities_by_user_id(7)
            .await
            .expect("list should succeed");
        assert_eq!(listed.len(), 1);
    }

    #[tokio::test]
    async fn find_identity_by_provider_email_normalizes_input() {
        let user_identity_repository = TestUserIdentityRepository::seeded(test_identity());
        let service = UserIdentityService::new(user_identity_repository);

        let found = service
            .find_identity_by_provider_email(IdentityProvider::Google, "  HELLO@example.com ")
            .await
            .expect("provider email lookup should succeed")
            .expect("identity should exist");

        assert_eq!(found.id, 5);
    }

    #[tokio::test]
    async fn record_authentication_updates_timestamp() {
        let user_identity_repository = TestUserIdentityRepository::seeded(test_identity());
        let service = UserIdentityService::new(user_identity_repository);

        let updated = service
            .record_authentication(5)
            .await
            .expect("record authentication should succeed");

        assert!(updated.last_authenticated_at.is_some());

        let found = service
            .find_identity_by_provider_subject(IdentityProvider::Google, "google-7")
            .await
            .expect("provider subject lookup should succeed")
            .expect("identity should exist");

        assert!(found.last_authenticated_at.is_some());
    }
}
