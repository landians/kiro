use sqlx::PgPool;

use crate::{
    application::{order::OrderLogic, product_purchase::ProductPurchaseLogic},
    infrastructure::persistence::{
        payment_order_repository::PaymentOrderRepository, product_repository::ProductRepository,
    },
};

pub fn build_order_logic(
    product_purchase_logic: ProductPurchaseLogic<ProductRepository>,
    pg_pool: PgPool,
) -> OrderLogic<ProductPurchaseLogic<ProductRepository>, PaymentOrderRepository> {
    let payment_order_repository = PaymentOrderRepository::new(pg_pool);

    OrderLogic::new(product_purchase_logic, payment_order_repository)
}
