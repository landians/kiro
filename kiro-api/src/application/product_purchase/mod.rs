use anyhow::Result;
use thiserror::Error;

use crate::domain::{
    entity::product::{Product, ProductPlan},
    repository::product_repository::ProductRepository,
};

pub struct ProductPurchaseLogic<PR> {
    product_repository: PR,
}

impl<PR> ProductPurchaseLogic<PR>
where
    PR: ProductRepository,
{
    pub fn new(product_repository: PR) -> Self {
        Self { product_repository }
    }
}

pub trait ProductPurchaseValidator: Send + Sync {
    async fn validate_plan_for_purchase(
        &self,
        plan_code: &str,
    ) -> Result<PurchasableProductContext>;
}

impl<PR> ProductPurchaseValidator for ProductPurchaseLogic<PR>
where
    PR: ProductRepository,
{
    #[tracing::instrument(skip(self), fields(plan.plan_code = plan_code))]
    async fn validate_plan_for_purchase(
        &self,
        plan_code: &str,
    ) -> Result<PurchasableProductContext> {
        let Some(plan) = self
            .product_repository
            .find_active_plan_by_code(plan_code)
            .await?
        else {
            return Err(ProductPurchaseLogicError::ProductPlanNotPurchasable {
                plan_code: plan_code.to_owned(),
            }
            .into());
        };

        let Some(product) = self
            .product_repository
            .find_active_product_by_id(plan.product_id)
            .await?
        else {
            return Err(ProductPurchaseLogicError::ProductNotPurchasable {
                product_id: plan.product_id,
                plan_code: plan.plan_code,
            }
            .into());
        };

        Ok(PurchasableProductContext { product, plan })
    }
}

pub struct PurchasableProductContext {
    pub product: Product,
    pub plan: ProductPlan,
}

#[derive(Debug, Error)]
pub enum ProductPurchaseLogicError {
    #[error("product plan {plan_code} is not purchasable")]
    ProductPlanNotPurchasable { plan_code: String },
    #[error("product {product_id} for plan {plan_code} is not purchasable")]
    ProductNotPurchasable { product_id: i64, plan_code: String },
}
