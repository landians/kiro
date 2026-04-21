use anyhow::{Context, Result};
use sqlx::{PgPool, Row, postgres::PgRow};

use crate::domain::{
    entity::product::{BillingInterval, CatalogStatus, ChargeType, Product, ProductPlan},
    repository::product_repository::ProductRepository as ProductRepositoryTrait,
};

#[derive(Clone)]
pub struct ProductRepository {
    pool: PgPool,
}

const LIST_ACTIVE_PRODUCTS_SQL: &str = r#"
    select
        id,
        product_code,
        product_name,
        product_description,
        product_status,
        created_at,
        updated_at
    from products
    where product_status = 'active'
    order by created_at desc, id desc
"#;

const FIND_ACTIVE_PRODUCT_BY_CODE_SQL: &str = r#"
    select
        id,
        product_code,
        product_name,
        product_description,
        product_status,
        created_at,
        updated_at
    from products
    where product_code = $1
      and product_status = 'active'
"#;

const LIST_ACTIVE_PRODUCT_PLANS_BY_PRODUCT_ID_SQL: &str = r#"
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
        billing_interval_count,
        trial_days,
        sort_order,
        is_default,
        created_at,
        updated_at
    from product_plans
    where product_id = $1
      and plan_status = 'active'
    order by sort_order asc, id asc
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
            billing_interval_count: row
                .try_get("billing_interval_count")
                .context("failed to decode product_plans.billing_interval_count")?,
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
}

impl ProductRepositoryTrait for ProductRepository {
    #[tracing::instrument(skip(self))]
    async fn list_active_products(&self) -> Result<Vec<Product>> {
        let rows = sqlx::query(LIST_ACTIVE_PRODUCTS_SQL)
            .fetch_all(&self.pool)
            .await
            .context("failed to query active products")?;

        rows.into_iter()
            .map(Self::map_product)
            .collect::<Result<Vec<_>>>()
    }

    #[tracing::instrument(skip(self), fields(product.product_code = product_code))]
    async fn find_active_product_by_code(&self, product_code: &str) -> Result<Option<Product>> {
        let row = sqlx::query(FIND_ACTIVE_PRODUCT_BY_CODE_SQL)
            .bind(product_code)
            .fetch_optional(&self.pool)
            .await
            .context("failed to query active product by code")?;

        row.map(Self::map_product).transpose()
    }

    #[tracing::instrument(skip(self), fields(product_id))]
    async fn list_active_plans_by_product_id(&self, product_id: i64) -> Result<Vec<ProductPlan>> {
        let rows = sqlx::query(LIST_ACTIVE_PRODUCT_PLANS_BY_PRODUCT_ID_SQL)
            .bind(product_id)
            .fetch_all(&self.pool)
            .await
            .context("failed to query active product plans by product id")?;

        rows.into_iter()
            .map(Self::map_product_plan)
            .collect::<Result<Vec<_>>>()
    }
}
