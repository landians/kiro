use serde::Serialize;

use crate::domain::entity::admin_user::AdminUser;

#[derive(Debug, Serialize)]
pub struct AdminUserDto {
    pub id: i64,
    pub email: String,
    pub display_name: Option<String>,
    pub account_status: &'static str,
}

impl From<AdminUser> for AdminUserDto {
    fn from(value: AdminUser) -> Self {
        Self {
            id: value.id,
            email: value.email,
            display_name: value.display_name,
            account_status: value.account_status.as_str(),
        }
    }
}
