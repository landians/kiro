use std::future::Future;

use thiserror::Error;

use crate::domain::account::{NewUser, User, UserError, UserStatus};

pub trait UserRepository: Clone + Send + Sync + 'static {
    fn create(
        &self,
        new_user: NewUser,
    ) -> impl Future<Output = Result<User, UserRepositoryError>> + Send;

    fn find_by_id(
        &self,
        user_id: i64,
    ) -> impl Future<Output = Result<Option<User>, UserRepositoryError>> + Send;

    fn find_by_user_code<'a>(
        &'a self,
        user_code: &'a str,
    ) -> impl Future<Output = Result<Option<User>, UserRepositoryError>> + Send + 'a;

    fn find_by_email_normalized<'a>(
        &'a self,
        email_normalized: &'a str,
    ) -> impl Future<Output = Result<Option<User>, UserRepositoryError>> + Send + 'a;

    fn update_status(
        &self,
        user_id: i64,
        next_status: UserStatus,
    ) -> impl Future<Output = Result<User, UserRepositoryError>> + Send;

    fn update_last_login_at(
        &self,
        user_id: i64,
        last_login_at: time::OffsetDateTime,
    ) -> impl Future<Output = Result<User, UserRepositoryError>> + Send;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserRepositoryError {
    #[error("user not found")]
    NotFound,
    #[error("user already exists for field `{field}`")]
    Conflict { field: &'static str },
    #[error(transparent)]
    Domain(#[from] UserError),
    #[error("{message}")]
    Unexpected { message: String },
}

impl UserRepositoryError {
    pub fn unexpected(message: impl Into<String>) -> Self {
        Self::Unexpected {
            message: message.into(),
        }
    }
}
