pub mod get_admin_user;

use anyhow::Result;
use thiserror::Error;

use self::get_admin_user::GetAdminUserLogic;
use crate::domain::{
    entity::admin_user::AdminUser, repository::admin_user_repository::AdminUserRepository,
};

pub struct AdminUserLogic<AR> {
    get_admin_user_logic: GetAdminUserLogic<AR>,
}

impl<AR> AdminUserLogic<AR>
where
    AR: AdminUserRepository + Clone,
{
    pub fn new(admin_user_repository: AR) -> Self {
        Self {
            get_admin_user_logic: GetAdminUserLogic::new(admin_user_repository),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn get(&self, admin_user_id: i64) -> Result<AdminUser> {
        self.get_admin_user_logic.execute(admin_user_id).await
    }
}

#[derive(Debug, Error)]
pub enum AdminUserLogicError {
    #[error("admin user {admin_user_id} not found")]
    AdminUserNotFound { admin_user_id: i64 },
}
