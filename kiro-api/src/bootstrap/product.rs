use sqlx::PgPool;

use crate::{
    application::product::ProductLogic,
    infrastructure::persistence::product_repository::ProductRepository,
};

pub fn build_product_logic(pg_pool: PgPool) -> ProductLogic<ProductRepository> {
    let product_repository = ProductRepository::new(pg_pool);

    ProductLogic::new(product_repository)
}
