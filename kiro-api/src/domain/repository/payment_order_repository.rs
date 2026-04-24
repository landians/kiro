#![allow(dead_code)]

use anyhow::Result;

use crate::domain::entity::{
    payment_order::{PaymentOrder, PaymentProvider},
    product::{BillingInterval, ChargeType},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePaymentOrder {
    pub order_no: String,
    pub user_id: i64,
    pub product_id: i64,
    pub product_plan_id: i64,
    pub payment_provider: PaymentProvider,
    pub product_code: String,
    pub product_name: String,
    pub product_image_url: Option<String>,
    pub plan_code: String,
    pub plan_name: String,
    pub charge_type: ChargeType,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<BillingInterval>,
    pub trial_days: i32,
}

pub trait PaymentOrderRepository: Send + Sync {
    async fn create(&self, order: CreatePaymentOrder) -> Result<PaymentOrder>;
}
