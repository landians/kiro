use serde::Serialize;

use crate::{
    application::product::ProductDetail,
    domain::entity::product::{Product, ProductPlan},
};

#[derive(Debug, Serialize)]
pub struct ProductListResponse {
    pub items: Vec<ProductSummaryDto>,
}

impl From<Vec<Product>> for ProductListResponse {
    fn from(value: Vec<Product>) -> Self {
        Self {
            items: value.into_iter().map(ProductSummaryDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductDetailResponse {
    pub product: ProductSummaryDto,
    pub plans: Vec<ProductPlanDto>,
}

impl From<ProductDetail> for ProductDetailResponse {
    fn from(value: ProductDetail) -> Self {
        Self {
            product: ProductSummaryDto::from(value.product),
            plans: value.plans.into_iter().map(ProductPlanDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductSummaryDto {
    pub product_code: String,
    pub product_name: String,
    pub product_description: Option<String>,
}

impl From<Product> for ProductSummaryDto {
    fn from(value: Product) -> Self {
        Self {
            product_code: value.product_code,
            product_name: value.product_name,
            product_description: value.product_description,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductPlanDto {
    pub plan_code: String,
    pub plan_name: String,
    pub charge_type: &'static str,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<&'static str>,
    pub billing_interval_count: Option<i32>,
    pub trial_days: i32,
    pub is_default: bool,
}

impl From<ProductPlan> for ProductPlanDto {
    fn from(value: ProductPlan) -> Self {
        Self {
            plan_code: value.plan_code,
            plan_name: value.plan_name,
            charge_type: value.charge_type.as_str(),
            currency_code: value.currency_code,
            amount_minor: value.amount_minor,
            billing_interval: value.billing_interval.map(|interval| interval.as_str()),
            billing_interval_count: value.billing_interval_count,
            trial_days: value.trial_days,
            is_default: value.is_default,
        }
    }
}
