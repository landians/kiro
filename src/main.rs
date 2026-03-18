use anyhow::{Ok, Result};
use dotenv::dotenv;

mod application;
mod domain;
mod infrastructure;
mod interfaces;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    Ok(())
}
