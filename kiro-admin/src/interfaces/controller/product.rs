use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, patch, post},
};
use validator::Validate;

use crate::{
    application::product::ProductLogicError,
    interfaces::{
        SharedState,
        dto::product::{
            CreateProductPlanRequest, CreateProductRequest, ListProductsRequest,
            ProductDetailResponse, ProductDto, ProductListResponse, ProductPlanDto,
            UpdateProductPlanRequest, UpdateProductRequest,
        },
        error::AppError,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_products).post(create_product))
        .route("/{product_id}", get(get_product).patch(update_product))
        .route("/{product_id}/plans", post(create_product_plan))
}

pub fn plan_routes() -> Router<SharedState> {
    Router::new().route("/product-plans/{plan_id}", patch(update_product_plan))
}

async fn list_products(
    State(state): State<SharedState>,
    Query(request): Query<ListProductsRequest>,
) -> Result<Json<ProductListResponse>, AppError> {
    request.validate()?;

    let products = state
        .product_logic()
        .list(request.into_query())
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductListResponse::from(products)))
}

async fn get_product(
    State(state): State<SharedState>,
    Path(product_id): Path<i64>,
) -> Result<Json<ProductDetailResponse>, AppError> {
    let product = state
        .product_logic()
        .get(product_id)
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductDetailResponse::from(product)))
}

async fn create_product(
    State(state): State<SharedState>,
    Json(request): Json<CreateProductRequest>,
) -> Result<Json<ProductDto>, AppError> {
    request.validate()?;

    let product = state
        .product_logic()
        .create(request.into_input())
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductDto::from(product)))
}

async fn update_product(
    State(state): State<SharedState>,
    Path(product_id): Path<i64>,
    Json(request): Json<UpdateProductRequest>,
) -> Result<Json<ProductDto>, AppError> {
    request.validate()?;

    let product = state
        .product_logic()
        .update(product_id, request.into_input())
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductDto::from(product)))
}

async fn create_product_plan(
    State(state): State<SharedState>,
    Path(product_id): Path<i64>,
    Json(request): Json<CreateProductPlanRequest>,
) -> Result<Json<ProductPlanDto>, AppError> {
    request.validate()?;

    let plan = state
        .product_logic()
        .create_plan(product_id, request.into_input())
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductPlanDto::from(plan)))
}

async fn update_product_plan(
    State(state): State<SharedState>,
    Path(plan_id): Path<i64>,
    Json(request): Json<UpdateProductPlanRequest>,
) -> Result<Json<ProductPlanDto>, AppError> {
    request.validate()?;

    let plan = state
        .product_logic()
        .update_plan(plan_id, request.into_input())
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductPlanDto::from(plan)))
}

struct ProductAppError(anyhow::Error);

impl From<anyhow::Error> for ProductAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<ProductAppError> for AppError {
    fn from(value: ProductAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<ProductLogicError>() {
            return match error {
                ProductLogicError::ProductNotFound { product_id } => AppError::not_found(
                    "product_not_found",
                    format!("product {product_id} not found"),
                ),
                ProductLogicError::ProductPlanNotFound { plan_id } => AppError::not_found(
                    "product_plan_not_found",
                    format!("product plan {plan_id} not found"),
                ),
                ProductLogicError::InvalidProductPlanConfiguration { reason } => {
                    AppError::bad_request("invalid_product_plan_configuration", reason.clone())
                }
            };
        }

        AppError::internal_server_error("product_logic_error", value.0.to_string())
    }
}
