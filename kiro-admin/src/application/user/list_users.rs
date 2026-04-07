use anyhow::Result;

use crate::domain::repository::user_repository::{ListUsers, PaginatedUsers, UserRepository};

pub struct ListUsersLogic<UR> {
    user_repository: UR,
}

impl<UR> ListUsersLogic<UR>
where
    UR: UserRepository,
{
    pub fn new(user_repository: UR) -> Self {
        Self { user_repository }
    }

    #[tracing::instrument(skip(self, query))]
    pub async fn execute(&self, query: ListUsers) -> Result<PaginatedUsers> {
        self.user_repository.list(&query).await
    }
}
