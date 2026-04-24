use axum::{Extension, Json, Router, extract::State, routing::post};
use validator::Validate;

use crate::{
    application::{order::CreateOrderInput, product_purchase::ProductPurchaseLogicError},
    interfaces::{
        SharedState,
        dto::order::{CreateOrderRequest, PaymentOrderDto},
        error::AppError,
        middleware::AuthenticatedUser,
    },
};

pub fn routes() -> Router<SharedState> {
    Router::new().route("/", post(create_order))
}

async fn create_order(
    State(state): State<SharedState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateOrderRequest>,
) -> Result<Json<PaymentOrderDto>, AppError> {
    request.validate()?;

    let input = CreateOrderInput {
        user_id: authenticated_user.user_id,
        plan_code: request.normalized_plan_code(),
        payment_provider: request.payment_provider(),
    };

    let order = state
        .order_logic()
        .create(input)
        .await
        .map_err(OrderAppError::from)
        .map_err(AppError::from)?;

    // TODO(payment):
    // After the local pending order is created, call the selected payment provider
    // adapter to create a checkout session, persist provider session identifiers on
    // payment_orders, and return checkout_url / checkout_session_id to the client
    // instead of only returning the local order snapshot.
    Ok(Json(PaymentOrderDto::from(order)))
}

struct OrderAppError(anyhow::Error);

impl From<anyhow::Error> for OrderAppError {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

impl From<OrderAppError> for AppError {
    fn from(value: OrderAppError) -> Self {
        if let Some(error) = value.0.downcast_ref::<ProductPurchaseLogicError>() {
            return match error {
                ProductPurchaseLogicError::ProductPlanNotPurchasable { plan_code } => {
                    AppError::bad_request(
                        "product_plan_not_purchasable",
                        format!("product plan {plan_code} is not purchasable"),
                    )
                }
                ProductPurchaseLogicError::ProductNotPurchasable {
                    product_id,
                    plan_code,
                } => AppError::bad_request(
                    "product_not_purchasable",
                    format!("product {product_id} for plan {plan_code} is not purchasable"),
                ),
            };
        }

        AppError::internal_server_error("create_order_error", value.0.to_string())
    }
}
