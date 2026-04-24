use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::domain::entity::payment_order::{PaymentOrder, PaymentProvider};

#[derive(Debug, Deserialize, Validate)]
#[validate(schema(function = "validate_create_order_request"))]
pub struct CreateOrderRequest {
    #[validate(length(min = 1, max = 64))]
    pub plan_code: String,
    pub payment_provider: Option<PaymentProviderQuery>,
}

impl CreateOrderRequest {
    pub fn normalized_plan_code(&self) -> String {
        self.plan_code.trim().to_lowercase()
    }

    pub fn payment_provider(&self) -> PaymentProvider {
        self.payment_provider
            .map(Into::into)
            .unwrap_or(PaymentProvider::Stripe)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentProviderQuery {
    Stripe,
    Creem,
}

impl From<PaymentProviderQuery> for PaymentProvider {
    fn from(value: PaymentProviderQuery) -> Self {
        match value {
            PaymentProviderQuery::Stripe => Self::Stripe,
            PaymentProviderQuery::Creem => Self::Creem,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaymentOrderDto {
    pub order_no: String,
    pub order_status: &'static str,
    pub payment_provider: &'static str,
    pub product_code: String,
    pub product_name: String,
    pub product_image_url: Option<String>,
    pub plan_code: String,
    pub plan_name: String,
    pub charge_type: &'static str,
    pub currency_code: String,
    pub amount_minor: i64,
    pub billing_interval: Option<&'static str>,
    pub trial_days: i32,
    pub created_at: String,
    // TODO(payment):
    // Extend this response with provider-facing checkout fields after Stripe / Creem
    // checkout creation is wired in, for example:
    // - checkout_url
    // - provider_checkout_session_id
    // - expires_at
}

impl From<PaymentOrder> for PaymentOrderDto {
    fn from(value: PaymentOrder) -> Self {
        Self {
            order_no: value.order_no,
            order_status: value.order_status.as_str(),
            payment_provider: value.payment_provider.as_str(),
            product_code: value.product_code,
            product_name: value.product_name,
            product_image_url: value.product_image_url,
            plan_code: value.plan_code,
            plan_name: value.plan_name,
            charge_type: value.charge_type.as_str(),
            currency_code: value.currency_code,
            amount_minor: value.amount_minor,
            billing_interval: value.billing_interval.map(|value| value.as_str()),
            trial_days: value.trial_days,
            created_at: value.created_at.to_rfc3339(),
        }
    }
}

fn validate_create_order_request(
    request: &CreateOrderRequest,
) -> Result<(), validator::ValidationError> {
    if request.plan_code.trim().is_empty() {
        let mut error = validator::ValidationError::new("blank_plan_code");
        error.message = Some("plan_code cannot be blank".into());
        return Err(error);
    }

    Ok(())
}
