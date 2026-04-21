use anyhow::Result;
use thiserror::Error;

use crate::domain::{
    entity::product::{Product, ProductPlan},
    repository::product_repository::ProductRepository,
};

pub struct ProductLogic<PR> {
    product_repository: PR,
}

impl<PR> ProductLogic<PR>
where
    PR: ProductRepository,
{
    pub fn new(product_repository: PR) -> Self {
        Self { product_repository }
    }

    #[tracing::instrument(skip(self))]
    pub async fn list(&self) -> Result<Vec<Product>> {
        self.product_repository.list_active_products().await
    }

    #[tracing::instrument(skip(self), fields(product.product_code = product_code))]
    pub async fn get(&self, product_code: &str) -> Result<ProductDetail> {
        let Some(product) = self
            .product_repository
            .find_active_product_by_code(product_code)
            .await?
        else {
            return Err(ProductLogicError::ProductNotFound {
                product_code: product_code.to_owned(),
            }
            .into());
        };

        let plans = self
            .product_repository
            .list_active_plans_by_product_id(product.id)
            .await?;

        Ok(ProductDetail { product, plans })
    }
}

pub struct ProductDetail {
    pub product: Product,
    pub plans: Vec<ProductPlan>,
}

#[derive(Debug, Error)]
pub enum ProductLogicError {
    #[error("product {product_code} not found")]
    ProductNotFound { product_code: String },
}
