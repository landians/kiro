use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};
use serde::Serialize;

use super::super::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(list_products))
        .route("/:product_id", get(get_product))
}

async fn list_products(State(_state): State<SharedState>) -> Json<ProductListResponse> {
    Json(ProductListResponse {
        message: "products collection route skeleton",
        items: Vec::new(),
    })
}

async fn get_product(
    State(_state): State<SharedState>,
    Path(product_id): Path<String>,
) -> Json<ProductDetailResponse> {
    Json(ProductDetailResponse {
        message: "product resource route skeleton",
        product_id,
    })
}

#[derive(Serialize)]
struct ProductListResponse {
    message: &'static str,
    items: Vec<ProductSummary>,
}

#[derive(Serialize)]
struct ProductSummary {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct ProductDetailResponse {
    message: &'static str,
    product_id: String,
}
