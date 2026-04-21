#![allow(dead_code)]

use anyhow::Result;

use crate::domain::entity::product::{Product, ProductPlan};

pub trait ProductRepository: Send + Sync {
    async fn list_active_products(&self) -> Result<Vec<Product>>;

    async fn find_active_product_by_code(&self, product_code: &str) -> Result<Option<Product>>;

    async fn list_active_plans_by_product_id(&self, product_id: i64) -> Result<Vec<ProductPlan>>;
}
