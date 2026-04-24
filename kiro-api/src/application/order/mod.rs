use anyhow::Result;
use ulid::Ulid;

use crate::{
    application::product_purchase::ProductPurchaseValidator,
    domain::{
        entity::payment_order::{PaymentOrder, PaymentProvider},
        repository::payment_order_repository::{CreatePaymentOrder, PaymentOrderRepository},
    },
};

pub struct OrderLogic<PV, OR> {
    product_purchase_validator: PV,
    payment_order_repository: OR,
}

impl<PV, OR> OrderLogic<PV, OR>
where
    PV: ProductPurchaseValidator,
    OR: PaymentOrderRepository,
{
    pub fn new(product_purchase_validator: PV, payment_order_repository: OR) -> Self {
        Self {
            product_purchase_validator,
            payment_order_repository,
        }
    }

    #[tracing::instrument(skip(self, input), fields(user_id = input.user_id, plan.plan_code = %input.plan_code))]
    pub async fn create(&self, input: CreateOrderInput) -> Result<PaymentOrder> {
        let purchasable = self
            .product_purchase_validator
            .validate_plan_for_purchase(&input.plan_code)
            .await?;
        let product = purchasable.product;
        let plan = purchasable.plan;

        let order = CreatePaymentOrder {
            order_no: generate_order_no(),
            user_id: input.user_id,
            product_id: product.id,
            product_plan_id: plan.id,
            payment_provider: input.payment_provider,
            product_code: product.product_code,
            product_name: product.product_name,
            product_image_url: product.product_image_url,
            plan_code: plan.plan_code,
            plan_name: plan.plan_name,
            charge_type: plan.charge_type,
            currency_code: plan.currency_code,
            amount_minor: plan.amount_minor,
            billing_interval: plan.billing_interval,
            trial_days: plan.trial_days,
        };

        let payment_order = self.payment_order_repository.create(order).await?;

        // TODO(payment):
        // Add a provider checkout orchestration step here.
        // 1. Resolve provider mapping for the selected internal product / plan.
        // 2. For Stripe, use the mapped price_id to create a Checkout Session.
        // 3. For Creem, use the mapped product_id to create a checkout.
        // 4. Persist provider_checkout_session_id / provider_customer_id / expires_at.
        // 5. Return checkout session data so the caller can redirect the user.
        //
        // This method currently stops at "create local pending order" and does not
        // yet initiate real payment with Stripe or Creem.
        Ok(payment_order)
    }
}

pub struct CreateOrderInput {
    pub user_id: i64,
    pub plan_code: String,
    pub payment_provider: PaymentProvider,
}

fn generate_order_no() -> String {
    format!("ord_{}", Ulid::new())
}
