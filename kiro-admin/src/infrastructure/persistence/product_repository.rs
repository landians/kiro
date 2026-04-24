use anyhow::{Context, Result};
use sqlx::{PgPool, Postgres, QueryBuilder, Row, postgres::PgRow};

use crate::domain::{
    entity::product::{BillingInterval, CatalogStatus, ChargeType, Product, ProductPlan},
    repository::product_repository::{
        CreateProduct, CreateProductPlan, ListProducts, PaginatedProducts,
        ProductRepository as ProductRepositoryTrait, UpdateProduct, UpdateProductPlan,
    },
};

#[derive(Clone)]
pub struct ProductRepository {
    pool: PgPool,
}

const PRODUCT_COLUMNS_SQL: &str = r#"
    id,
    product_code,
    product_name,
    product_description,
    product_image_url,
    product_status,
    created_at,
    updated_at
"#;

const FIND_PRODUCT_BY_ID_SQL: &str = r#"
    select
        id,
        product_code,
        product_name,
        product_description,
        product_image_url,
        product_status,
        created_at,
        updated_at
    from products
    where id = $1
"#;

const CREATE_PRODUCT_SQL: &str = r#"
    insert into products (
        product_code,
        product_name,
        product_description,
        product_image_url,
        product_status
    )
    values ($1, $2, $3, $4, $5)
    returning
        id,
        product_code,
        product_name,
        product_description,
        product_image_url,
        product_status,
        created_at,
        updated_at
"#;

const UPDATE_PRODUCT_SQL: &str = r#"
    update products
    set
        product_name = $2,
        product_description = $3,
        product_image_url = $4,
        product_status = $5,
        updated_at = now()
    where id = $1
    returning
        id,
        product_code,
        product_name,
        product_description,
        product_image_url,
        product_status,
        created_at,
        updated_at
"#;

const LIST_PRODUCT_PLANS_BY_PRODUCT_ID_SQL: &str = r#"
    select
        id,
        product_id,
        plan_code,
        plan_name,
        plan_status,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days,
        sort_order,
        is_default,
        created_at,
        updated_at
    from product_plans
    where product_id = $1
    order by sort_order asc, id asc
"#;

const FIND_PRODUCT_PLAN_BY_ID_SQL: &str = r#"
    select
        id,
        product_id,
        plan_code,
        plan_name,
        plan_status,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days,
        sort_order,
        is_default,
        created_at,
        updated_at
    from product_plans
    where id = $1
"#;

const CREATE_PRODUCT_PLAN_SQL: &str = r#"
    insert into product_plans (
        product_id,
        plan_code,
        plan_name,
        plan_status,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days,
        sort_order,
        is_default
    )
    values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    returning
        id,
        product_id,
        plan_code,
        plan_name,
        plan_status,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days,
        sort_order,
        is_default,
        created_at,
        updated_at
"#;

const UPDATE_PRODUCT_PLAN_SQL: &str = r#"
    update product_plans
    set
        plan_name = $2,
        plan_status = $3,
        charge_type = $4,
        currency_code = $5,
        amount_minor = $6,
        billing_interval = $7,
        trial_days = $8,
        sort_order = $9,
        is_default = $10,
        updated_at = now()
    where id = $1
    returning
        id,
        product_id,
        plan_code,
        plan_name,
        plan_status,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days,
        sort_order,
        is_default,
        created_at,
        updated_at
"#;

impl ProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_product(row: PgRow) -> Result<Product> {
        let product_status = row
            .try_get::<String, _>("product_status")
            .context("failed to decode products.product_status")?;

        Ok(Product {
            id: row.try_get("id").context("failed to decode products.id")?,
            product_code: row
                .try_get("product_code")
                .context("failed to decode products.product_code")?,
            product_name: row
                .try_get("product_name")
                .context("failed to decode products.product_name")?,
            product_description: row
                .try_get("product_description")
                .context("failed to decode products.product_description")?,
            product_image_url: row
                .try_get("product_image_url")
                .context("failed to decode products.product_image_url")?,
            product_status: CatalogStatus::from_db(&product_status)?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode products.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode products.updated_at")?,
        })
    }

    fn map_product_plan(row: PgRow) -> Result<ProductPlan> {
        let plan_status = row
            .try_get::<String, _>("plan_status")
            .context("failed to decode product_plans.plan_status")?;
        let charge_type = row
            .try_get::<String, _>("charge_type")
            .context("failed to decode product_plans.charge_type")?;
        let billing_interval = row
            .try_get::<Option<String>, _>("billing_interval")
            .context("failed to decode product_plans.billing_interval")?;

        Ok(ProductPlan {
            id: row
                .try_get("id")
                .context("failed to decode product_plans.id")?,
            product_id: row
                .try_get("product_id")
                .context("failed to decode product_plans.product_id")?,
            plan_code: row
                .try_get("plan_code")
                .context("failed to decode product_plans.plan_code")?,
            plan_name: row
                .try_get("plan_name")
                .context("failed to decode product_plans.plan_name")?,
            plan_status: CatalogStatus::from_db(&plan_status)?,
            charge_type: ChargeType::from_db(&charge_type)?,
            currency_code: row
                .try_get("currency_code")
                .context("failed to decode product_plans.currency_code")?,
            amount_minor: row
                .try_get("amount_minor")
                .context("failed to decode product_plans.amount_minor")?,
            billing_interval: billing_interval
                .as_deref()
                .map(BillingInterval::from_db)
                .transpose()?,
            trial_days: row
                .try_get("trial_days")
                .context("failed to decode product_plans.trial_days")?,
            sort_order: row
                .try_get("sort_order")
                .context("failed to decode product_plans.sort_order")?,
            is_default: row
                .try_get("is_default")
                .context("failed to decode product_plans.is_default")?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode product_plans.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode product_plans.updated_at")?,
        })
    }

    fn push_filters(builder: &mut QueryBuilder<'_, Postgres>, query: &ListProducts) {
        let mut has_where = false;

        if let Some(product_code) = query.product_code.as_deref() {
            Self::push_filter_prefix(builder, &mut has_where);
            builder
                .push("product_code ilike ")
                .push_bind(format!("%{product_code}%"));
        }

        if let Some(product_name) = query.product_name.as_deref() {
            Self::push_filter_prefix(builder, &mut has_where);
            builder
                .push("product_name ilike ")
                .push_bind(format!("%{product_name}%"));
        }

        if let Some(product_status) = query.product_status {
            Self::push_filter_prefix(builder, &mut has_where);
            builder
                .push("product_status = ")
                .push_bind(product_status.as_str());
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

impl ProductRepositoryTrait for ProductRepository {
    #[tracing::instrument(skip(self, query))]
    async fn list(&self, query: &ListProducts) -> Result<PaginatedProducts> {
        let limit = i64::try_from(query.page_size).context("page_size exceeds i64")?;
        let offset = i64::try_from(query.offset()).context("offset exceeds i64")?;

        let mut count_query = QueryBuilder::new("select count(*) as total from products");
        Self::push_filters(&mut count_query, query);

        let count_row = count_query
            .build()
            .fetch_one(&self.pool)
            .await
            .context("failed to count products")?;
        let total = count_row
            .try_get::<i64, _>("total")
            .context("failed to decode products total count")?;
        let total = u64::try_from(total).context("products total count cannot be negative")?;

        let mut list_query =
            QueryBuilder::new(format!("select {PRODUCT_COLUMNS_SQL} from products"));
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
            .context("failed to query products")?;
        let items = rows
            .into_iter()
            .map(Self::map_product)
            .collect::<Result<Vec<_>>>()?;

        Ok(PaginatedProducts {
            items,
            total,
            page: query.page,
            page_size: query.page_size,
        })
    }

    #[tracing::instrument(skip(self), fields(product_id = id))]
    async fn find_by_id(&self, id: i64) -> Result<Option<Product>> {
        let row = sqlx::query(FIND_PRODUCT_BY_ID_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query product by id")?;

        row.map(Self::map_product).transpose()
    }

    #[tracing::instrument(skip(self, product), fields(product.product_code = %product.product_code))]
    async fn create(&self, product: CreateProduct) -> Result<Product> {
        let row = sqlx::query(CREATE_PRODUCT_SQL)
            .bind(product.product_code)
            .bind(product.product_name)
            .bind(product.product_description)
            .bind(product.product_image_url)
            .bind(product.product_status.as_str())
            .fetch_one(&self.pool)
            .await
            .context("failed to insert product")?;

        Self::map_product(row)
    }

    #[tracing::instrument(skip(self, product), fields(product_id = id))]
    async fn update(&self, id: i64, product: UpdateProduct) -> Result<Product> {
        let row = sqlx::query(UPDATE_PRODUCT_SQL)
            .bind(id)
            .bind(product.product_name)
            .bind(product.product_description)
            .bind(product.product_image_url)
            .bind(product.product_status.as_str())
            .fetch_one(&self.pool)
            .await
            .context("failed to update product")?;

        Self::map_product(row)
    }

    #[tracing::instrument(skip(self), fields(product_id))]
    async fn list_plans_by_product_id(&self, product_id: i64) -> Result<Vec<ProductPlan>> {
        let rows = sqlx::query(LIST_PRODUCT_PLANS_BY_PRODUCT_ID_SQL)
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .context("failed to query product plans by product id")?;

        rows.into_iter()
            .map(Self::map_product_plan)
            .collect::<Result<Vec<_>>>()
    }

    #[tracing::instrument(skip(self), fields(plan_id = id))]
    async fn find_plan_by_id(&self, id: i64) -> Result<Option<ProductPlan>> {
        let row = sqlx::query(FIND_PRODUCT_PLAN_BY_ID_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query product plan by id")?;

        row.map(Self::map_product_plan).transpose()
    }

    #[tracing::instrument(skip(self, plan), fields(product_id = plan.product_id, plan.plan_code = %plan.plan_code))]
    async fn create_plan(&self, plan: CreateProductPlan) -> Result<ProductPlan> {
        let row = sqlx::query(CREATE_PRODUCT_PLAN_SQL)
            .bind(plan.product_id)
            .bind(plan.plan_code)
            .bind(plan.plan_name)
            .bind(plan.plan_status.as_str())
            .bind(plan.charge_type.as_str())
            .bind(plan.currency_code)
            .bind(plan.amount_minor)
            .bind(plan.billing_interval.map(|value| value.as_str()))
            .bind(plan.trial_days)
            .bind(plan.sort_order)
            .bind(plan.is_default)
            .fetch_one(&self.pool)
            .await
            .context("failed to insert product plan")?;

        Self::map_product_plan(row)
    }

    #[tracing::instrument(skip(self, plan), fields(plan_id = id))]
    async fn update_plan(&self, id: i64, plan: UpdateProductPlan) -> Result<ProductPlan> {
        let row = sqlx::query(UPDATE_PRODUCT_PLAN_SQL)
            .bind(id)
            .bind(plan.plan_name)
            .bind(plan.plan_status.as_str())
            .bind(plan.charge_type.as_str())
            .bind(plan.currency_code)
            .bind(plan.amount_minor)
            .bind(plan.billing_interval.map(|value| value.as_str()))
            .bind(plan.trial_days)
            .bind(plan.sort_order)
            .bind(plan.is_default)
            .fetch_one(&self.pool)
            .await
            .context("failed to update product plan")?;

        Self::map_product_plan(row)
    }
}
