use anyhow::Result;

use super::AdminUserLogicError;
use crate::domain::{
    entity::admin_user::AdminUser, repository::admin_user_repository::AdminUserRepository,
};

pub struct GetAdminUserLogic<AR> {
    admin_user_repository: AR,
}

impl<AR> GetAdminUserLogic<AR>
where
    AR: AdminUserRepository,
{
    pub fn new(admin_user_repository: AR) -> Self {
        Self {
            admin_user_repository,
        }
    }

    pub async fn execute(&self, admin_user_id: i64) -> Result<AdminUser> {
        let Some(admin_user) = self.admin_user_repository.find_by_id(admin_user_id).await? else {
            return Err(AdminUserLogicError::AdminUserNotFound { admin_user_id }.into());
        };

        Ok(admin_user)
    }
}
