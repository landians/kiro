create table kiro.payment_orders (
    id bigint generated always as identity,
    order_code varchar(64) not null,
    user_id bigint not null,
    plan_id bigint not null,
    price_id bigint not null,
    provider varchar(32) not null,
    order_status varchar(32) not null default 'pending',
    currency_code char(3) not null,
    amount_minor bigint not null,
    idempotency_key varchar(128) not null,
    provider_customer_id varchar(128),
    provider_order_id varchar(128),
    paid_at timestamptz,
    expires_at timestamptz,
    failure_code varchar(64),
    failure_message varchar(256),
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_payment_orders primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete restrict
        on update cascade,
    foreign key (plan_id) references kiro.product_plans (id)
        on delete restrict
        on update cascade,
    foreign key (price_id) references kiro.product_prices (id)
        on delete restrict
        on update cascade,
    check (provider in ('stripe', 'creem')),
    check (order_status in (
        'pending',
        'requires_action',
        'processing',
        'succeeded',
        'failed',
        'canceled',
        'expired',
        'refunded'
    )),
    check (amount_minor >= 0)
);

create unique index uk_payment_orders_code on kiro.payment_orders (order_code);
create unique index uk_payment_orders_provider_oid
    on kiro.payment_orders (provider, provider_order_id)
    where provider_order_id is not null;
create unique index uk_payment_orders_user_idem
    on kiro.payment_orders (user_id, idempotency_key);
create index idx_payment_orders_user_status_time
    on kiro.payment_orders (user_id, order_status, created_at desc);
create index idx_payment_orders_plan_id on kiro.payment_orders (plan_id);

create table kiro.payment_attempts (
    id bigint generated always as identity,
    attempt_code varchar(64) not null,
    payment_order_id bigint not null,
    provider varchar(32) not null,
    attempt_status varchar(32) not null default 'created',
    request_idempotency_key varchar(128) not null,
    provider_attempt_id varchar(128),
    checkout_url varchar(1024),
    requested_at timestamptz not null default now(),
    completed_at timestamptz,
    failure_code varchar(64),
    failure_message varchar(256),
    request_payload_jsonb jsonb not null default '{}'::jsonb,
    response_payload_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_payment_attempts primary key (id),
    foreign key (payment_order_id) references kiro.payment_orders (id)
        on delete cascade
        on update cascade,
    check (provider in ('stripe', 'creem')),
    check (attempt_status in (
        'created',
        'submitted',
        'requires_action',
        'succeeded',
        'failed',
        'canceled',
        'expired'
    ))
);

create unique index uk_payment_attempts_code on kiro.payment_attempts (attempt_code);
create unique index uk_payment_attempts_order_idem
    on kiro.payment_attempts (payment_order_id, request_idempotency_key);
create unique index uk_payment_attempts_provider_aid
    on kiro.payment_attempts (provider, provider_attempt_id)
    where provider_attempt_id is not null;
create index idx_payment_attempts_order_status
    on kiro.payment_attempts (payment_order_id, attempt_status, requested_at desc);

create table kiro.payment_callbacks (
    id bigint generated always as identity,
    callback_code varchar(64) not null,
    payment_order_id bigint,
    provider varchar(32) not null,
    provider_event_id varchar(128) not null,
    callback_type varchar(64) not null,
    signature_valid boolean not null default false,
    process_status varchar(32) not null default 'pending',
    received_at timestamptz not null default now(),
    processed_at timestamptz,
    error_message varchar(256),
    payload_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    constraint pk_payment_callbacks primary key (id),
    foreign key (payment_order_id) references kiro.payment_orders (id)
        on delete set null
        on update cascade,
    check (provider in ('stripe', 'creem')),
    check (process_status in ('pending', 'processed', 'ignored', 'failed'))
);

create unique index uk_payment_callbacks_code on kiro.payment_callbacks (callback_code);
create unique index uk_payment_callbacks_provider_eid
    on kiro.payment_callbacks (provider, provider_event_id);
create index idx_payment_callbacks_order_status
    on kiro.payment_callbacks (payment_order_id, process_status, received_at desc);
create index idx_payment_callbacks_status_time
    on kiro.payment_callbacks (process_status, received_at desc);

create table kiro.subscriptions (
    id bigint generated always as identity,
    subscription_code varchar(64) not null,
    user_id bigint not null,
    product_id bigint not null,
    plan_id bigint not null,
    latest_payment_order_id bigint,
    subscription_status varchar(32) not null default 'pending',
    renewal_status varchar(32) not null default 'auto_renew',
    current_period_start_at timestamptz,
    current_period_end_at timestamptz,
    trial_end_at timestamptz,
    canceled_at timestamptz,
    expires_at timestamptz,
    cancellation_reason varchar(128),
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_subscriptions primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete restrict
        on update cascade,
    foreign key (product_id) references kiro.products (id)
        on delete restrict
        on update cascade,
    foreign key (plan_id) references kiro.product_plans (id)
        on delete restrict
        on update cascade,
    foreign key (latest_payment_order_id) references kiro.payment_orders (id)
        on delete set null
        on update cascade,
    check (subscription_status in (
        'pending',
        'trialing',
        'active',
        'past_due',
        'canceled',
        'expired',
        'paused'
    )),
    check (renewal_status in (
        'auto_renew',
        'cancel_at_period_end',
        'manual',
        'stopped'
    ))
);

create unique index uk_subscriptions_code on kiro.subscriptions (subscription_code);
create unique index uk_subscriptions_active
    on kiro.subscriptions (user_id, plan_id)
    where subscription_status in ('pending', 'trialing', 'active', 'past_due', 'paused');
create index idx_subscriptions_user_status_time
    on kiro.subscriptions (user_id, subscription_status, created_at desc);
create index idx_subscriptions_latest_order
    on kiro.subscriptions (latest_payment_order_id)
    where latest_payment_order_id is not null;

create table kiro.subscription_periods (
    id bigint generated always as identity,
    subscription_period_code varchar(64) not null,
    subscription_id bigint not null,
    period_index integer not null,
    source_payment_order_id bigint,
    period_start_at timestamptz not null,
    period_end_at timestamptz not null,
    charge_status varchar(32) not null default 'pending',
    period_status varchar(32) not null default 'scheduled',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_subscription_periods primary key (id),
    foreign key (subscription_id) references kiro.subscriptions (id)
        on delete cascade
        on update cascade,
    foreign key (source_payment_order_id) references kiro.payment_orders (id)
        on delete set null
        on update cascade,
    check (period_index > 0),
    check (period_end_at > period_start_at),
    check (charge_status in ('pending', 'paid', 'failed', 'waived', 'refunded')),
    check (period_status in ('scheduled', 'active', 'completed', 'canceled', 'expired'))
);

create unique index uk_subscription_periods_code
    on kiro.subscription_periods (subscription_period_code);
create unique index uk_subscription_periods_index
    on kiro.subscription_periods (subscription_id, period_index);
create index idx_subscription_periods_time
    on kiro.subscription_periods (subscription_id, period_start_at desc);

create table kiro.subscription_events (
    id bigint generated always as identity,
    subscription_event_code varchar(64) not null,
    subscription_id bigint not null,
    payment_order_id bigint,
    event_type varchar(64) not null,
    event_status varchar(32) not null default 'applied',
    idempotency_key varchar(128),
    trace_id varchar(64),
    payload_jsonb jsonb not null default '{}'::jsonb,
    occurred_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    constraint pk_subscription_events primary key (id),
    foreign key (subscription_id) references kiro.subscriptions (id)
        on delete cascade
        on update cascade,
    foreign key (payment_order_id) references kiro.payment_orders (id)
        on delete set null
        on update cascade,
    check (event_status in ('pending', 'applied', 'ignored', 'failed'))
);

create unique index uk_subscription_events_code
    on kiro.subscription_events (subscription_event_code);
create unique index uk_subscription_events_idem
    on kiro.subscription_events (subscription_id, idempotency_key)
    where idempotency_key is not null;
create index idx_subscription_events_time
    on kiro.subscription_events (subscription_id, occurred_at desc);
