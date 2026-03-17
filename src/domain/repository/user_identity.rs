use std::future::Future;

use thiserror::Error;

use crate::domain::user_identity::{
    IdentityProvider, NewUserIdentity, UserIdentity, UserIdentityError,
};

pub trait UserIdentityRepository: Clone + Send + Sync + 'static {
    fn create(
        &self,
        new_identity: NewUserIdentity,
    ) -> impl Future<Output = Result<UserIdentity, UserIdentityRepositoryError>> + Send;

    fn find_by_id(
        &self,
        identity_id: i64,
    ) -> impl Future<Output = Result<Option<UserIdentity>, UserIdentityRepositoryError>> + Send;

    fn find_by_identity_code<'a>(
        &'a self,
        identity_code: &'a str,
    ) -> impl Future<Output = Result<Option<UserIdentity>, UserIdentityRepositoryError>> + Send + 'a;

    fn find_by_provider_subject<'a>(
        &'a self,
        provider: IdentityProvider,
        provider_user_id: &'a str,
    ) -> impl Future<Output = Result<Option<UserIdentity>, UserIdentityRepositoryError>> + Send + 'a;

    fn find_by_provider_email_normalized<'a>(
        &'a self,
        provider: IdentityProvider,
        provider_email_normalized: &'a str,
    ) -> impl Future<Output = Result<Option<UserIdentity>, UserIdentityRepositoryError>> + Send + 'a;

    fn list_by_user_id(
        &self,
        user_id: i64,
    ) -> impl Future<Output = Result<Vec<UserIdentity>, UserIdentityRepositoryError>> + Send;

    fn update_last_authenticated_at(
        &self,
        identity_id: i64,
        last_authenticated_at: time::OffsetDateTime,
    ) -> impl Future<Output = Result<UserIdentity, UserIdentityRepositoryError>> + Send;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserIdentityRepositoryError {
    #[error("user identity not found")]
    NotFound,
    #[error("user identity already exists for field `{field}`")]
    Conflict { field: &'static str },
    #[error(transparent)]
    Domain(#[from] UserIdentityError),
    #[error("{message}")]
    Unexpected { message: String },
}

impl UserIdentityRepositoryError {
    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected {
            message: message.into(),
        }
    }
}
