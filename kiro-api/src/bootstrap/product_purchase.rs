use sqlx::PgPool;

use crate::{
    application::product_purchase::ProductPurchaseLogic,
    infrastructure::persistence::product_repository::ProductRepository,
};

pub fn build_product_purchase_logic(pg_pool: PgPool) -> ProductPurchaseLogic<ProductRepository> {
    let product_repository = ProductRepository::new(pg_pool);

    ProductPurchaseLogic::new(product_repository)
}
