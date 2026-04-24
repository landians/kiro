use anyhow::Result;
use thiserror::Error;

use crate::domain::{
    entity::product::{BillingInterval, CatalogStatus, ChargeType, Product, ProductPlan},
    repository::product_repository::{
        CreateProduct, CreateProductPlan, ListProducts, PaginatedProducts, ProductRepository,
        UpdateProduct, UpdateProductPlan,
    },
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

    #[tracing::instrument(skip(self, query))]
    pub async fn list(&self, query: ListProducts) -> Result<PaginatedProducts> {
        self.product_repository.list(&query).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get(&self, product_id: i64) -> Result<ProductDetail> {
        let Some(product) = self.product_repository.find_by_id(product_id).await? else {
            return Err(ProductLogicError::ProductNotFound { product_id }.into());
        };
        let plans = self
            .product_repository
            .list_plans_by_product_id(product_id)
            .await?;

        Ok(ProductDetail { product, plans })
    }

    #[tracing::instrument(skip(self, input))]
    pub async fn create(&self, input: CreateProductInput) -> Result<Product> {
        self.product_repository
            .create(CreateProduct {
                product_code: input.product_code,
                product_name: input.product_name,
                product_description: input.product_description,
                product_image_url: input.product_image_url,
                product_status: input.product_status,
            })
            .await
    }

    #[tracing::instrument(skip(self, input))]
    pub async fn update(&self, product_id: i64, input: UpdateProductInput) -> Result<Product> {
        let Some(current_product) = self.product_repository.find_by_id(product_id).await? else {
            return Err(ProductLogicError::ProductNotFound { product_id }.into());
        };

        let product = UpdateProduct {
            product_name: input.product_name.unwrap_or(current_product.product_name),
            product_description: input
                .product_description
                .or(current_product.product_description),
            product_image_url: input
                .product_image_url
                .or(current_product.product_image_url),
            product_status: input
                .product_status
                .unwrap_or(current_product.product_status),
        };

        self.product_repository.update(product_id, product).await
    }

    #[tracing::instrument(skip(self, input))]
    pub async fn create_plan(
        &self,
        product_id: i64,
        input: CreateProductPlanInput,
    ) -> Result<ProductPlan> {
        let Some(_product) = self.product_repository.find_by_id(product_id).await? else {
            return Err(ProductLogicError::ProductNotFound { product_id }.into());
        };

        self.validate_plan_configuration(
            input.charge_type,
            input.billing_interval,
            input.trial_days,
        )?;

        let plan = CreateProductPlan {
            product_id,
            plan_code: input.plan_code,
            plan_name: input.plan_name,
            plan_status: input.plan_status,
            charge_type: input.charge_type,
            currency_code: input.currency_code,
            amount_minor: input.amount_minor,
            billing_interval: input.billing_interval,
            trial_days: input.trial_days,
            sort_order: input.sort_order,
            is_default: input.is_default,
        };

        self.product_repository.create_plan(plan).await
    }

    #[tracing::instrument(skip(self, input))]
    pub async fn update_plan(
        &self,
        plan_id: i64,
        input: UpdateProductPlanInput,
    ) -> Result<ProductPlan> {
        let Some(current_plan) = self.product_repository.find_plan_by_id(plan_id).await? else {
            return Err(ProductLogicError::ProductPlanNotFound { plan_id }.into());
        };

        let merged_charge_type = input.charge_type.unwrap_or(current_plan.charge_type);
        let merged_billing_interval = input.billing_interval.or(current_plan.billing_interval);
        let merged_trial_days = input.trial_days.unwrap_or(current_plan.trial_days);

        self.validate_plan_configuration(
            merged_charge_type,
            merged_billing_interval,
            merged_trial_days,
        )?;

        let plan = UpdateProductPlan {
            plan_name: input.plan_name.unwrap_or(current_plan.plan_name),
            plan_status: input.plan_status.unwrap_or(current_plan.plan_status),
            charge_type: merged_charge_type,
            currency_code: input.currency_code.unwrap_or(current_plan.currency_code),
            amount_minor: input.amount_minor.unwrap_or(current_plan.amount_minor),
            billing_interval: merged_billing_interval,
            trial_days: merged_trial_days,
            sort_order: input.sort_order.unwrap_or(current_plan.sort_order),
            is_default: input.is_default.unwrap_or(current_plan.is_default),
        };

        self.product_repository.update_plan(plan_id, plan).await
    }

    fn validate_plan_configuration(
        &self,
        charge_type: ChargeType,
        billing_interval: Option<BillingInterval>,
        trial_days: i32,
    ) -> Result<()> {
        match charge_type {
            ChargeType::Subscription => {
                if billing_interval.is_none() {
                    return Err(ProductLogicError::InvalidProductPlanConfiguration {
                        reason: "subscription plan requires billing_interval".to_owned(),
                    }
                    .into());
                }
            }
            ChargeType::OneTime => {
                if billing_interval.is_some() {
                    return Err(ProductLogicError::InvalidProductPlanConfiguration {
                        reason: "one_time plan cannot set billing_interval".to_owned(),
                    }
                    .into());
                }

                if trial_days != 0 {
                    return Err(ProductLogicError::InvalidProductPlanConfiguration {
                        reason: "one_time plan cannot set trial_days".to_owned(),
                    }
                    .into());
                }
            }
        }

        Ok(())
    }
}

pub struct ProductDetail {
    pub product: Product,
    pub plans: Vec<ProductPlan>,
}

pub struct CreateProductInput {
    pub product_code: String,
    pub product_name: String,
    pub product_description: Option<String>,
    pub product_image_url: Option<String>,
    pub product_status: CatalogStatus,
}

pub struct UpdateProductInput {
    pub product_name: Option<String>,
    pub product_description: Option<String>,
    pub product_image_url: Option<String>,
    pub product_status: Option<CatalogStatus>,
}

pub struct CreateProductPlanInput {
    pub plan_code: String,
    pub plan_name: String,
    pub plan_status: CatalogStatus,
    pub charge_type: ChargeType,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<BillingInterval>,
    pub trial_days: i32,
    pub sort_order: i32,
    pub is_default: bool,
}

pub struct UpdateProductPlanInput {
    pub plan_name: Option<String>,
    pub plan_status: Option<CatalogStatus>,
    pub charge_type: Option<ChargeType>,
    pub currency_code: Option<String>,
    pub amount_minor: Option<i64>,
    pub billing_interval: Option<BillingInterval>,
    pub trial_days: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Error)]
pub enum ProductLogicError {
    #[error("product {product_id} not found")]
    ProductNotFound { product_id: i64 },
    #[error("product plan {plan_id} not found")]
    ProductPlanNotFound { plan_id: i64 },
    #[error("{reason}")]
    InvalidProductPlanConfiguration { reason: String },
}
