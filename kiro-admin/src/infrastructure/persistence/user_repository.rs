use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, postgres::PgRow};

use crate::domain::{
    entity::user::{AccountStatus, User},
    repository::user_repository::{
        ListUsers, PaginatedUsers, UpdateUserStatus, UserRepository as UserRepositoryTrait,
    },
};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

const USER_COLUMNS_SQL: &str = r#"
    id,
    primary_email,
    email_verified,
    display_name,
    avatar_url,
    account_status,
    frozen_at,
    banned_at,
    last_login_at,
    created_at,
    updated_at
"#;

const FIND_USER_BY_ID_SQL: &str = r#"
    select
        id,
        primary_email,
        email_verified,
        display_name,
        avatar_url,
        account_status,
        frozen_at,
        banned_at,
        last_login_at,
        created_at,
        updated_at
    from users
    where id = $1
"#;

const UPDATE_USER_STATUS_SQL: &str = r#"
    update users
    set
        account_status = $2,
        frozen_at = $3,
        banned_at = $4,
        updated_at = now()
    where id = $1
    returning
        id,
        primary_email,
        email_verified,
        display_name,
        avatar_url,
        account_status,
        frozen_at,
        banned_at,
        last_login_at,
        created_at,
        updated_at
"#;

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_user(row: PgRow) -> Result<User> {
        let account_status = row
            .try_get::<String, _>("account_status")
            .context("failed to decode users.account_status")?;

        Ok(User {
            id: row.try_get("id").context("failed to decode users.id")?,
            primary_email: row
                .try_get("primary_email")
                .context("failed to decode users.primary_email")?,
            email_verified: row
                .try_get("email_verified")
                .context("failed to decode users.email_verified")?,
            display_name: row
                .try_get("display_name")
                .context("failed to decode users.display_name")?,
            avatar_url: row
                .try_get("avatar_url")
                .context("failed to decode users.avatar_url")?,
            account_status: AccountStatus::from_db(&account_status)?,
            frozen_at: row
                .try_get("frozen_at")
                .context("failed to decode users.frozen_at")?,
            banned_at: row
                .try_get("banned_at")
                .context("failed to decode users.banned_at")?,
            last_login_at: row
                .try_get("last_login_at")
                .context("failed to decode users.last_login_at")?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode users.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode users.updated_at")?,
        })
    }

    fn push_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &ListUsers) {
        let mut has_where = false;

        if let Some(uid) = query.uid {
            Self::push_filter_prefix(builder, &mut has_where);
            builder.push("id = ").push_bind(uid);
        }

        if let Some(user_name) = query.user_name.as_deref() {
            Self::push_filter_prefix(builder, &mut has_where);
            builder
                .push("display_name ilike ")
                .push_bind(format!("%{user_name}%"));
        }

        if let Some(account_status) = query.account_status {
            Self::push_filter_prefix(builder, &mut has_where);
            builder
                .push("account_status = ")
                .push_bind(account_status.as_str());
        }
    }

    fn push_filter_prefix(builder: &mut QueryBuilder<'_, Postgres>, has_where: &mut bool) {
        if *has_where {
            builder.push(" and ");
            return;
        }

        builder.push(" where ");
        *has_where = true;
    }
}

impl UserRepositoryTrait for UserRepository {
    #[tracing::instrument(skip(self, query))]
    async fn list(&self, query: &ListUsers) -> Result<PaginatedUsers> {
        let limit = i64::try_from(query.page_size).context("page_size exceeds i64")?;
        let offset = i64::try_from(query.offset()).context("offset exceeds i64")?;

        let mut count_query = QueryBuilder::new("select count(*) as total from users");
        Self::push_filters(&mut count_query, query);

        let count_row = count_query
            .build()
            .fetch_one(&self.pool)
            .await
            .context("failed to count users")?;
        let total = count_row
            .try_get::<i64, _>("total")
            .context("failed to decode users total count")?;
        let total = u64::try_from(total).context("users total count cannot be negative")?;

        let mut list_query = QueryBuilder::new(format!("select {USER_COLUMNS_SQL} from users"));
        Self::push_filters(&mut list_query, query);
        list_query
            .push(" order by created_at desc, id desc")
            .push(" limit ")
            .push_bind(limit)
            .push(" offset ")
            .push_bind(offset);

        let rows = list_query
            .build()
            .fetch_all(&self.pool)
            .await
            .context("failed to query users")?;
        let items = rows
            .into_iter()
            .map(Self::map_user)
            .collect::<Result<Vec<_>>>()?;

        Ok(PaginatedUsers {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    #[tracing::instrument(skip(self), fields(user_id = id))]
    async fn find_by_id(&self, id: i64) -> Result<Option<User>> {
        let row = sqlx::query(FIND_USER_BY_ID_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query user by id")?;

        row.map(Self::map_user).transpose()
    }

    #[tracing::instrument(skip(self, update), fields(user_id = id, user.account_status = %update.account_status))]
    async fn update_status(&self, id: i64, update: UpdateUserStatus) -> Result<User> {
        let row = sqlx::query(UPDATE_USER_STATUS_SQL)
            .bind(id)
            .bind(update.account_status.as_str())
            .bind(update.frozen_at)
            .bind(update.banned_at)
            .fetch_one(&self.pool)
            .await
            .context("failed to update user status")?;

        Self::map_user(row)
    }
}
