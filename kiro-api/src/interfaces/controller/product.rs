use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

use crate::{
    application::product::ProductLogicError,
    interfaces::{
        SharedState,
        dto::product::{ProductDetailResponse, ProductListResponse},
        error::AppError,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_products))
        .route("/{product_code}", get(get_product))
}

async fn list_products(
    State(state): State<SharedState>,
) -> Result<Json<ProductListResponse>, AppError> {
    let products = state
        .product_logic()
        .list()
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductListResponse::from(products)))
}

async fn get_product(
    State(state): State<SharedState>,
    Path(product_code): Path<String>,
) -> Result<Json<ProductDetailResponse>, AppError> {
    let product = state
        .product_logic()
        .get(&product_code)
        .await
        .map_err(ProductAppError::from)
        .map_err(AppError::from)?;

    Ok(Json(ProductDetailResponse::from(product)))
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
                ProductLogicError::ProductNotFound { product_code } => AppError::not_found(
                    "product_not_found",
                    format!("product {product_code} not found"),
                ),
            };
        }

        AppError::internal_server_error("product_logic_error", value.0.to_string())
    }
}
