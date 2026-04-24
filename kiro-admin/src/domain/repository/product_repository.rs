#![allow(dead_code)]

use anyhow::Result;

use crate::domain::entity::product::{
    BillingInterval, CatalogStatus, ChargeType, Product, ProductPlan,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListProducts {
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub product_status: Option<CatalogStatus>,
    pub page: u64,
    pub page_size: u64,
}

impl ListProducts {
    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.page_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginatedProducts {
    pub items: Vec<Product>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProduct {
    pub product_code: String,
    pub product_name: String,
    pub product_description: Option<String>,
    pub product_image_url: Option<String>,
    pub product_status: CatalogStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProduct {
    pub product_name: String,
    pub product_description: Option<String>,
    pub product_image_url: Option<String>,
    pub product_status: CatalogStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateProductPlan {
    pub product_id: i64,
    pub plan_code: String,
    pub plan_name: String,
    pub plan_status: CatalogStatus,
    pub charge_type: ChargeType,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<BillingInterval>,
    pub billing_interval_count: Option<i32>,
    pub trial_days: i32,
    pub sort_order: i32,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProductPlan {
    pub plan_name: String,
    pub plan_status: CatalogStatus,
    pub charge_type: ChargeType,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<BillingInterval>,
    pub billing_interval_count: Option<i32>,
    pub trial_days: i32,
    pub sort_order: i32,
    pub is_default: bool,
}

pub trait ProductRepository: Send + Sync {
    async fn list(&self, query: &ListProducts) -> Result<PaginatedProducts>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Product>>;
    async fn create(&self, product: CreateProduct) -> Result<Product>;
    async fn update(&self, id: i64, product: UpdateProduct) -> Result<Product>;
    async fn list_plans_by_product_id(&self, product_id: i64) -> Result<Vec<ProductPlan>>;
    async fn find_plan_by_id(&self, id: i64) -> Result<Option<ProductPlan>>;
    async fn create_plan(&self, plan: CreateProductPlan) -> Result<ProductPlan>;
    async fn update_plan(&self, id: i64, plan: UpdateProductPlan) -> Result<ProductPlan>;
}
