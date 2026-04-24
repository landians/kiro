create table payment_orders (
    id bigint generated always as identity,
    order_no varchar(64) not null,
    user_id bigint not null,
    product_id bigint not null,
    product_plan_id bigint not null,
    payment_provider varchar(32) not null,
    order_status varchar(32) not null default 'pending',
    provider_checkout_session_id varchar(255) null,
    provider_payment_id varchar(255) null,
    provider_customer_id varchar(255) null,
    product_code varchar(64) not null,
    product_name varchar(128) not null,
    product_image_url text null,
    plan_code varchar(64) not null,
    plan_name varchar(128) not null,
    charge_type varchar(32) not null,
    currency_code varchar(3) not null,
    amount_minor bigint not null,
    billing_interval varchar(16) null,
    trial_days integer not null default 0,
    failure_code varchar(64) null,
    failure_message text null,
    expires_at timestamptz null,
    paid_at timestamptz null,
    failed_at timestamptz null,
    canceled_at timestamptz null,
    refunded_at timestamptz null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_payment_orders primary key (id),
    constraint uk_payment_orders_order_no unique (order_no),
    constraint fk_payment_orders_user_id
        foreign key (user_id)
        references users (id)
        on delete restrict
        on update restrict,
    constraint fk_payment_orders_product_id
        foreign key (product_id)
        references products (id)
        on delete restrict
        on update restrict,
    constraint fk_payment_orders_product_plan_id
        foreign key (product_plan_id)
        references product_plans (id)
        on delete restrict
        on update restrict,
    constraint ck_payment_orders_payment_provider check (
        payment_provider in ('stripe', 'creem')
    ),
    constraint ck_payment_orders_order_status check (
        order_status in ('pending', 'paid', 'failed', 'canceled', 'refunded')
    ),
    constraint ck_payment_orders_charge_type check (
        charge_type in ('one_time', 'subscription')
    ),
    constraint ck_payment_orders_currency_code check (
        currency_code = upper(currency_code)
        and char_length(currency_code) = 3
    ),
    constraint ck_payment_orders_amount_minor check (
        amount_minor >= 0
    ),
    constraint ck_payment_orders_trial_days check (
        trial_days >= 0
    ),
    constraint ck_payment_orders_billing_interval check (
        billing_interval in ('month', 'year')
        or billing_interval is null
    ),
    constraint ck_payment_orders_subscription_fields check (
        (
            charge_type = 'subscription'
            and billing_interval is not null
        )
        or
        (
            charge_type = 'one_time'
            and billing_interval is null
            and trial_days = 0
        )
    )
);

create index idx_payment_orders_user_id_created_at
    on payment_orders (user_id, created_at desc);

create index idx_payment_orders_order_status_created_at
    on payment_orders (order_status, created_at desc);

create unique index uk_payment_orders_provider_checkout_session_id
    on payment_orders (payment_provider, provider_checkout_session_id)
    where provider_checkout_session_id is not null;

create unique index uk_payment_orders_provider_payment_id
    on payment_orders (payment_provider, provider_payment_id)
    where provider_payment_id is not null;
