use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::{
    application::product::{
        CreateProductInput, CreateProductPlanInput, ProductDetail, UpdateProductInput,
        UpdateProductPlanInput,
    },
    domain::{
        entity::product::{BillingInterval, CatalogStatus, ChargeType, Product, ProductPlan},
        repository::product_repository::{ListProducts, PaginatedProducts},
    },
};

#[derive(Debug, Deserialize, Validate)]
pub struct ListProductsRequest {
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub product_status: Option<CatalogStatusQuery>,
    #[validate(range(min = 1, max = 10_000))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100))]
    pub page_size: Option<u64>,
}

impl ListProductsRequest {
    const DEFAULT_PAGE: u64 = 1;
    const DEFAULT_PAGE_SIZE: u64 = 20;

    pub fn into_query(self) -> ListProducts {
        ListProducts {
            product_code: normalize_optional_text(self.product_code),
            product_name: normalize_optional_text(self.product_name),
            product_status: self.product_status.map(Into::into),
            page: self.page.unwrap_or(Self::DEFAULT_PAGE),
            page_size: self.page_size.unwrap_or(Self::DEFAULT_PAGE_SIZE),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_create_product_request"))]
pub struct CreateProductRequest {
    #[validate(length(max = 64))]
    pub product_code: String,
    #[validate(length(max = 128))]
    pub product_name: String,
    pub product_description: Option<String>,
    #[validate(url)]
    pub product_image_url: Option<String>,
    pub product_status: Option<CatalogStatusQuery>,
}

impl CreateProductRequest {
    pub fn into_input(self) -> CreateProductInput {
        CreateProductInput {
            product_code: self.product_code.trim().to_lowercase(),
            product_name: self.product_name.trim().to_owned(),
            product_description: normalize_optional_text(self.product_description),
            product_image_url: normalize_optional_text(self.product_image_url),
            product_status: self
                .product_status
                .map(Into::into)
                .unwrap_or(CatalogStatus::Draft),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_update_product_request"))]
pub struct UpdateProductRequest {
    #[validate(length(max = 128))]
    pub product_name: Option<String>,
    pub product_description: Option<String>,
    #[validate(url)]
    pub product_image_url: Option<String>,
    pub product_status: Option<CatalogStatusQuery>,
}

impl UpdateProductRequest {
    pub fn into_input(self) -> UpdateProductInput {
        UpdateProductInput {
            product_name: self.product_name.map(|value| value.trim().to_owned()),
            product_description: normalize_optional_text(self.product_description),
            product_image_url: normalize_optional_text(self.product_image_url),
            product_status: self.product_status.map(Into::into),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_create_product_plan_request"))]
pub struct CreateProductPlanRequest {
    #[validate(length(max = 64))]
    pub plan_code: String,
    #[validate(length(max = 128))]
    pub plan_name: String,
    pub plan_status: Option<CatalogStatusQuery>,
    pub charge_type: ChargeTypeQuery,
    #[validate(length(equal = 3))]
    pub currency_code: String,
    #[validate(range(min = 0))]
    pub amount_minor: i64,
    pub billing_interval: Option<BillingIntervalQuery>,
    #[validate(range(min = 0))]
    pub trial_days: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_default: Option<bool>,
}

impl CreateProductPlanRequest {
    pub fn into_input(self) -> CreateProductPlanInput {
        CreateProductPlanInput {
            plan_code: self.plan_code.trim().to_lowercase(),
            plan_name: self.plan_name.trim().to_owned(),
            plan_status: self
                .plan_status
                .map(Into::into)
                .unwrap_or(CatalogStatus::Draft),
            charge_type: self.charge_type.into(),
            currency_code: self.currency_code.trim().to_uppercase(),
            amount_minor: self.amount_minor,
            billing_interval: self.billing_interval.map(Into::into),
            trial_days: self.trial_days.unwrap_or(0),
            sort_order: self.sort_order.unwrap_or(0),
            is_default: self.is_default.unwrap_or(false),
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_update_product_plan_request"))]
pub struct UpdateProductPlanRequest {
    #[validate(length(max = 128))]
    pub plan_name: Option<String>,
    pub plan_status: Option<CatalogStatusQuery>,
    pub charge_type: Option<ChargeTypeQuery>,
    #[validate(length(equal = 3))]
    pub currency_code: Option<String>,
    #[validate(range(min = 0))]
    pub amount_minor: Option<i64>,
    pub billing_interval: Option<BillingIntervalQuery>,
    #[validate(range(min = 0))]
    pub trial_days: Option<i32>,
    pub sort_order: Option<i32>,
    pub is_default: Option<bool>,
}

impl UpdateProductPlanRequest {
    pub fn into_input(self) -> UpdateProductPlanInput {
        UpdateProductPlanInput {
            plan_name: self.plan_name.map(|value| value.trim().to_owned()),
            plan_status: self.plan_status.map(Into::into),
            charge_type: self.charge_type.map(Into::into),
            currency_code: self.currency_code.map(|value| value.trim().to_uppercase()),
            amount_minor: self.amount_minor,
            billing_interval: self.billing_interval.map(Into::into),
            trial_days: self.trial_days,
            sort_order: self.sort_order,
            is_default: self.is_default,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogStatusQuery {
    Draft,
    Active,
    Inactive,
    Archived,
}

impl From<CatalogStatusQuery> for CatalogStatus {
    fn from(value: CatalogStatusQuery) -> Self {
        match value {
            CatalogStatusQuery::Draft => Self::Draft,
            CatalogStatusQuery::Active => Self::Active,
            CatalogStatusQuery::Inactive => Self::Inactive,
            CatalogStatusQuery::Archived => Self::Archived,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargeTypeQuery {
    OneTime,
    Subscription,
}

impl From<ChargeTypeQuery> for ChargeType {
    fn from(value: ChargeTypeQuery) -> Self {
        match value {
            ChargeTypeQuery::OneTime => Self::OneTime,
            ChargeTypeQuery::Subscription => Self::Subscription,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingIntervalQuery {
    Month,
    Year,
}

impl From<BillingIntervalQuery> for BillingInterval {
    fn from(value: BillingIntervalQuery) -> Self {
        match value {
            BillingIntervalQuery::Month => Self::Month,
            BillingIntervalQuery::Year => Self::Year,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductListResponse {
    pub items: Vec<ProductDto>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
}

impl From<PaginatedProducts> for ProductListResponse {
    fn from(value: PaginatedProducts) -> Self {
        Self {
            items: value.items.into_iter().map(ProductDto::from).collect(),
            page: value.page,
            page_size: value.page_size,
            total: value.total,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductDetailResponse {
    pub product: ProductDto,
    pub plans: Vec<ProductPlanDto>,
}

impl From<ProductDetail> for ProductDetailResponse {
    fn from(value: ProductDetail) -> Self {
        Self {
            product: ProductDto::from(value.product),
            plans: value.plans.into_iter().map(ProductPlanDto::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductDto {
    pub id: i64,
    pub product_code: String,
    pub product_name: String,
    pub product_description: Option<String>,
    pub product_image_url: Option<String>,
    pub product_status: &'static str,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Product> for ProductDto {
    fn from(value: Product) -> Self {
        Self {
            id: value.id,
            product_code: value.product_code,
            product_name: value.product_name,
            product_description: value.product_description,
            product_image_url: value.product_image_url,
            product_status: value.product_status.as_str(),
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ProductPlanDto {
    pub id: i64,
    pub product_id: i64,
    pub plan_code: String,
    pub plan_name: String,
    pub plan_status: &'static str,
    pub charge_type: &'static str,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<&'static str>,
    pub trial_days: i32,
    pub sort_order: i32,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ProductPlan> for ProductPlanDto {
    fn from(value: ProductPlan) -> Self {
        Self {
            id: value.id,
            product_id: value.product_id,
            plan_code: value.plan_code,
            plan_name: value.plan_name,
            plan_status: value.plan_status.as_str(),
            charge_type: value.charge_type.as_str(),
            currency_code: value.currency_code,
            amount_minor: value.amount_minor,
            billing_interval: value.billing_interval.map(|value| value.as_str()),
            trial_days: value.trial_days,
            sort_order: value.sort_order,
            is_default: value.is_default,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
        }
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        if value.is_empty() {
            return None;
        }

        Some(value)
    })
}

fn validate_create_product_request(
    request: &CreateProductRequest,
) -> Result<(), validator::ValidationError> {
    if request.product_code.trim().is_empty() {
        let mut error = validator::ValidationError::new("blank_product_code");
        error.message = Some("product_code cannot be blank".into());
        return Err(error);
    }

    if request.product_name.trim().is_empty() {
        let mut error = validator::ValidationError::new("blank_product_name");
        error.message = Some("product_name cannot be blank".into());
        return Err(error);
    }

    Ok(())
}

fn validate_update_product_request(
    request: &UpdateProductRequest,
) -> Result<(), validator::ValidationError> {
    if request.product_name.is_none()
        && request.product_description.is_none()
        && request.product_image_url.is_none()
        && request.product_status.is_none()
    {
        let mut error = validator::ValidationError::new("empty_product_update");
        error.message = Some("at least one updatable field is required".into());
        return Err(error);
    }

    if let Some(product_name) = request.product_name.as_deref()
        && product_name.trim().is_empty()
    {
        let mut error = validator::ValidationError::new("blank_product_name");
        error.message = Some("product_name cannot be blank".into());
        return Err(error);
    }

    Ok(())
}

fn validate_create_product_plan_request(
    request: &CreateProductPlanRequest,
) -> Result<(), validator::ValidationError> {
    if request.plan_code.trim().is_empty() {
        let mut error = validator::ValidationError::new("blank_plan_code");
        error.message = Some("plan_code cannot be blank".into());
        return Err(error);
    }

    if request.plan_name.trim().is_empty() {
        let mut error = validator::ValidationError::new("blank_plan_name");
        error.message = Some("plan_name cannot be blank".into());
        return Err(error);
    }

    if matches!(request.charge_type, ChargeTypeQuery::Subscription)
        && request.billing_interval.is_none()
    {
        let mut error = validator::ValidationError::new("missing_subscription_billing_fields");
        error.message = Some("subscription plan requires billing_interval".into());
        return Err(error);
    }

    if matches!(request.charge_type, ChargeTypeQuery::OneTime)
        && (request.billing_interval.is_some() || request.trial_days.unwrap_or(0) != 0)
    {
        let mut error = validator::ValidationError::new("invalid_one_time_plan_fields");
        error.message =
            Some("one_time plan cannot set billing_interval or non-zero trial_days".into());
        return Err(error);
    }

    Ok(())
}

fn validate_update_product_plan_request(
    request: &UpdateProductPlanRequest,
) -> Result<(), validator::ValidationError> {
    if request.plan_name.is_none()
        && request.plan_status.is_none()
        && request.charge_type.is_none()
        && request.currency_code.is_none()
        && request.amount_minor.is_none()
        && request.billing_interval.is_none()
        && request.trial_days.is_none()
        && request.sort_order.is_none()
        && request.is_default.is_none()
    {
        let mut error = validator::ValidationError::new("empty_product_plan_update");
        error.message = Some("at least one updatable field is required".into());
        return Err(error);
    }

    if let Some(plan_name) = request.plan_name.as_deref()
        && plan_name.trim().is_empty()
    {
        let mut error = validator::ValidationError::new("blank_plan_name");
        error.message = Some("plan_name cannot be blank".into());
        return Err(error);
    }

    Ok(())
}
