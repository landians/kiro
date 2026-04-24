use anyhow::{Context, Result};
use sqlx::{PgPool, Row, postgres::PgRow};

use crate::domain::{
    entity::{
        payment_order::{PaymentOrder, PaymentOrderStatus, PaymentProvider},
        product::{BillingInterval, ChargeType},
    },
    repository::payment_order_repository::{
        CreatePaymentOrder, PaymentOrderRepository as PaymentOrderRepositoryTrait,
    },
};

#[derive(Clone)]
pub struct PaymentOrderRepository {
    pool: PgPool,
}

const CREATE_PAYMENT_ORDER_SQL: &str = r#"
    insert into payment_orders (
        order_no,
        user_id,
        product_id,
        product_plan_id,
        payment_provider,
        order_status,
        product_code,
        product_name,
        product_image_url,
        plan_code,
        plan_name,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days
    )
    values (
        $1, $2, $3, $4, $5, 'pending', $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
    )
    returning
        id,
        order_no,
        user_id,
        product_id,
        product_plan_id,
        payment_provider,
        order_status,
        provider_checkout_session_id,
        provider_payment_id,
        provider_customer_id,
        product_code,
        product_name,
        product_image_url,
        plan_code,
        plan_name,
        charge_type,
        currency_code,
        amount_minor,
        billing_interval,
        trial_days,
        failure_code,
        failure_message,
        expires_at,
        paid_at,
        failed_at,
        canceled_at,
        refunded_at,
        created_at,
        updated_at
"#;

impl PaymentOrderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_payment_order(row: PgRow) -> Result<PaymentOrder> {
        let payment_provider = row
            .try_get::<String, _>("payment_provider")
            .context("failed to decode payment_orders.payment_provider")?;
        let order_status = row
            .try_get::<String, _>("order_status")
            .context("failed to decode payment_orders.order_status")?;
        let charge_type = row
            .try_get::<String, _>("charge_type")
            .context("failed to decode payment_orders.charge_type")?;
        let billing_interval = row
            .try_get::<Option<String>, _>("billing_interval")
            .context("failed to decode payment_orders.billing_interval")?;

        Ok(PaymentOrder {
            id: row
                .try_get("id")
                .context("failed to decode payment_orders.id")?,
            order_no: row
                .try_get("order_no")
                .context("failed to decode payment_orders.order_no")?,
            user_id: row
                .try_get("user_id")
                .context("failed to decode payment_orders.user_id")?,
            product_id: row
                .try_get("product_id")
                .context("failed to decode payment_orders.product_id")?,
            product_plan_id: row
                .try_get("product_plan_id")
                .context("failed to decode payment_orders.product_plan_id")?,
            payment_provider: PaymentProvider::from_db(&payment_provider)?,
            order_status: PaymentOrderStatus::from_db(&order_status)?,
            provider_checkout_session_id: row
                .try_get("provider_checkout_session_id")
                .context("failed to decode payment_orders.provider_checkout_session_id")?,
            provider_payment_id: row
                .try_get("provider_payment_id")
                .context("failed to decode payment_orders.provider_payment_id")?,
            provider_customer_id: row
                .try_get("provider_customer_id")
                .context("failed to decode payment_orders.provider_customer_id")?,
            product_code: row
                .try_get("product_code")
                .context("failed to decode payment_orders.product_code")?,
            product_name: row
                .try_get("product_name")
                .context("failed to decode payment_orders.product_name")?,
            product_image_url: row
                .try_get("product_image_url")
                .context("failed to decode payment_orders.product_image_url")?,
            plan_code: row
                .try_get("plan_code")
                .context("failed to decode payment_orders.plan_code")?,
            plan_name: row
                .try_get("plan_name")
                .context("failed to decode payment_orders.plan_name")?,
            charge_type: ChargeType::from_db(&charge_type)?,
            currency_code: row
                .try_get("currency_code")
                .context("failed to decode payment_orders.currency_code")?,
            amount_minor: row
                .try_get("amount_minor")
                .context("failed to decode payment_orders.amount_minor")?,
            billing_interval: billing_interval
                .as_deref()
                .map(BillingInterval::from_db)
                .transpose()?,
            trial_days: row
                .try_get("trial_days")
                .context("failed to decode payment_orders.trial_days")?,
            failure_code: row
                .try_get("failure_code")
                .context("failed to decode payment_orders.failure_code")?,
            failure_message: row
                .try_get("failure_message")
                .context("failed to decode payment_orders.failure_message")?,
            expires_at: row
                .try_get("expires_at")
                .context("failed to decode payment_orders.expires_at")?,
            paid_at: row
                .try_get("paid_at")
                .context("failed to decode payment_orders.paid_at")?,
            failed_at: row
                .try_get("failed_at")
                .context("failed to decode payment_orders.failed_at")?,
            canceled_at: row
                .try_get("canceled_at")
                .context("failed to decode payment_orders.canceled_at")?,
            refunded_at: row
                .try_get("refunded_at")
                .context("failed to decode payment_orders.refunded_at")?,
            created_at: row
                .try_get("created_at")
                .context("failed to decode payment_orders.created_at")?,
            updated_at: row
                .try_get("updated_at")
                .context("failed to decode payment_orders.updated_at")?,
        })
    }
}

impl PaymentOrderRepositoryTrait for PaymentOrderRepository {
    #[tracing::instrument(skip(self, order), fields(order.order_no = %order.order_no, user_id = order.user_id))]
    async fn create(&self, order: CreatePaymentOrder) -> Result<PaymentOrder> {
        let row = sqlx::query(CREATE_PAYMENT_ORDER_SQL)
            .bind(order.order_no)
            .bind(order.user_id)
            .bind(order.product_id)
            .bind(order.product_plan_id)
            .bind(order.payment_provider.as_str())
            .bind(order.product_code)
            .bind(order.product_name)
            .bind(order.product_image_url)
            .bind(order.plan_code)
            .bind(order.plan_name)
            .bind(order.charge_type.as_str())
            .bind(order.currency_code)
            .bind(order.amount_minor)
            .bind(order.billing_interval.map(|value| value.as_str()))
            .bind(order.trial_days)
            .fetch_one(&self.pool)
            .await
            .context("failed to insert payment order")?;

        Self::map_payment_order(row)
    }
}
