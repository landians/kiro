use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::{
    application::user::UserLogicError,
    domain::{
        entity::user::{AccountStatus, User},
        repository::user_repository::{UpdateUserStatus, UserRepository},
    },
};

pub struct UpdateUserStatusLogic<UR> {
    user_repository: UR,
}

impl<UR> UpdateUserStatusLogic<UR>
where
    UR: UserRepository,
{
    pub fn new(user_repository: UR) -> Self {
        Self { user_repository }
    }

    #[tracing::instrument(skip(self))]
    pub async fn freeze(&self, user_id: i64, frozen_at: DateTime<Utc>) -> Result<User> {
        let user = self.load_user(user_id).await?;
        self.ensure_transition_allowed(&user, AccountStatus::Frozen)?;

        let update = UpdateUserStatus {
            account_status: AccountStatus::Frozen,
            frozen_at: Some(frozen_at),
            banned_at: user.banned_at,
        };

        self.user_repository.update_status(user_id, update).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn ban(&self, user_id: i64, banned_at: DateTime<Utc>) -> Result<User> {
        let user = self.load_user(user_id).await?;
        self.ensure_transition_allowed(&user, AccountStatus::Banned)?;

        let update = UpdateUserStatus {
            account_status: AccountStatus::Banned,
            frozen_at: user.frozen_at,
            banned_at: Some(banned_at),
        };

        self.user_repository.update_status(user_id, update).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn activate(&self, user_id: i64) -> Result<User> {
        let user = self.load_user(user_id).await?;
        self.ensure_transition_allowed(&user, AccountStatus::Active)?;

        let update = UpdateUserStatus {
            account_status: AccountStatus::Active,
            frozen_at: None,
            banned_at: None,
        };

        self.user_repository.update_status(user_id, update).await
    }

    async fn load_user(&self, user_id: i64) -> Result<User> {
        let Some(user) = self.user_repository.find_by_id(user_id).await? else {
            return Err(UserLogicError::UserNotFound { user_id }.into());
        };

        Ok(user)
    }

    fn ensure_transition_allowed(&self, user: &User, target_status: AccountStatus) -> Result<()> {
        if user.account_status == target_status {
            return Err(UserLogicError::InvalidUserStatusTransition {
                user_id: user.id,
                current_status: user.account_status,
                target_status,
            }
            .into());
        }

        if user.is_banned() && target_status == AccountStatus::Frozen {
            return Err(UserLogicError::InvalidUserStatusTransition {
                user_id: user.id,
                current_status: user.account_status,
                target_status,
            }
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use chrono::{TimeZone, Utc};

    use super::UpdateUserStatusLogic;
    use crate::{
        application::user::UserLogicError,
        domain::{
            entity::user::{AccountStatus, User},
            repository::user_repository::{
                ListUsers, PaginatedUsers, UpdateUserStatus, UserRepository,
            },
        },
    };

    struct FakeUserRepository {
        user: Mutex<Option<User>>,
    }

    impl FakeUserRepository {
        fn new(user: User) -> Self {
            Self {
                user: Mutex::new(Some(user)),
            }
        }
    }

    impl UserRepository for FakeUserRepository {
        async fn list(&self, _query: &ListUsers) -> anyhow::Result<PaginatedUsers> {
            unreachable!("list is not used in status transition tests");
        }

        async fn find_by_id(&self, id: i64) -> anyhow::Result<Option<User>> {
            let user = self.user.lock().expect("mutex poisoned");
            Ok(user.as_ref().filter(|user| user.id == id).cloned())
        }

        async fn update_status(&self, id: i64, update: UpdateUserStatus) -> anyhow::Result<User> {
            let mut user = self.user.lock().expect("mutex poisoned");
            let current_user = user.as_mut().expect("user should exist in fake repo");

            current_user.id = id;
            current_user.account_status = update.account_status;
            current_user.frozen_at = update.frozen_at;
            current_user.banned_at = update.banned_at;
            current_user.updated_at = Utc::now();

            Ok(current_user.clone())
        }
    }

    #[tokio::test]
    async fn freeze_active_user_sets_frozen_status_and_timestamp() {
        let logic = UpdateUserStatusLogic::new(FakeUserRepository::new(
            build_user(AccountStatus::Active).build(),
        ));
        let frozen_at = Utc.with_ymd_and_hms(2026, 4, 21, 8, 30, 0).unwrap();

        let user = logic
            .freeze(42, frozen_at)
            .await
            .expect("freeze should succeed");

        assert_eq!(user.account_status, AccountStatus::Frozen);
        assert_eq!(user.frozen_at, Some(frozen_at));
        assert_eq!(user.banned_at, None);
    }

    #[tokio::test]
    async fn ban_frozen_user_preserves_frozen_timestamp() {
        let frozen_at = Utc.with_ymd_and_hms(2026, 4, 20, 9, 0, 0).unwrap();
        let banned_at = Utc.with_ymd_and_hms(2026, 4, 21, 10, 0, 0).unwrap();
        let logic = UpdateUserStatusLogic::new(FakeUserRepository::new(
            build_user(AccountStatus::Frozen).with_frozen_at(frozen_at),
        ));

        let user = logic.ban(42, banned_at).await.expect("ban should succeed");

        assert_eq!(user.account_status, AccountStatus::Banned);
        assert_eq!(user.frozen_at, Some(frozen_at));
        assert_eq!(user.banned_at, Some(banned_at));
    }

    #[tokio::test]
    async fn freeze_banned_user_is_rejected() {
        let logic = UpdateUserStatusLogic::new(FakeUserRepository::new(
            build_user(AccountStatus::Banned).build(),
        ));
        let frozen_at = Utc.with_ymd_and_hms(2026, 4, 21, 8, 30, 0).unwrap();

        let error = logic
            .freeze(42, frozen_at)
            .await
            .expect_err("freeze should fail");
        let error = error
            .downcast_ref::<UserLogicError>()
            .expect("should downcast to UserLogicError");

        assert!(matches!(
            error,
            UserLogicError::InvalidUserStatusTransition {
                current_status: AccountStatus::Banned,
                target_status: AccountStatus::Frozen,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn activate_frozen_user_clears_status_markers() {
        let frozen_at = Utc.with_ymd_and_hms(2026, 4, 20, 9, 0, 0).unwrap();
        let banned_at = Utc.with_ymd_and_hms(2026, 4, 21, 10, 0, 0).unwrap();
        let logic = UpdateUserStatusLogic::new(FakeUserRepository::new(
            build_user(AccountStatus::Frozen)
                .with_frozen_at_value(frozen_at)
                .with_banned_at_value(banned_at),
        ));

        let user = logic.activate(42).await.expect("activate should succeed");

        assert_eq!(user.account_status, AccountStatus::Active);
        assert_eq!(user.frozen_at, None);
        assert_eq!(user.banned_at, None);
    }

    fn build_user(account_status: AccountStatus) -> TestUserBuilder {
        TestUserBuilder::new(account_status)
    }

    struct TestUserBuilder {
        user: User,
    }

    impl TestUserBuilder {
        fn new(account_status: AccountStatus) -> Self {
            let now = Utc.with_ymd_and_hms(2026, 4, 21, 8, 0, 0).unwrap();

            Self {
                user: User {
                    id: 42,
                    primary_email: Some("user@example.com".to_owned()),
                    email_verified: true,
                    display_name: Some("tester".to_owned()),
                    avatar_url: None,
                    account_status,
                    frozen_at: None,
                    banned_at: None,
                    last_login_at: None,
                    created_at: now,
                    updated_at: now,
                },
            }
        }

        fn with_frozen_at(mut self, frozen_at: chrono::DateTime<Utc>) -> User {
            self.user.frozen_at = Some(frozen_at);
            self.user
        }

        fn with_frozen_at_value(mut self, frozen_at: chrono::DateTime<Utc>) -> Self {
            self.user.frozen_at = Some(frozen_at);
            self
        }

        fn with_banned_at_value(mut self, banned_at: chrono::DateTime<Utc>) -> User {
            self.user.banned_at = Some(banned_at);
            self.user
        }

        fn build(self) -> User {
            self.user
        }
    }
}
