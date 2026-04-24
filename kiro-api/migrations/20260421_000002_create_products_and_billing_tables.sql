create table products (
    id bigint generated always as identity,
    product_code varchar(64) not null,
    product_name varchar(128) not null,
    product_image_url text null,
    product_description text null,
    product_status varchar(32) not null default 'draft',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_products primary key (id),
    constraint uk_products_product_code unique (product_code),
    constraint ck_products_product_status check (
        product_status in ('draft', 'active', 'inactive', 'archived')
    )
);

create table product_plans (
    id bigint generated always as identity,
    product_id bigint not null,
    plan_code varchar(64) not null,
    plan_name varchar(128) not null,
    plan_status varchar(32) not null default 'draft',
    charge_type varchar(32) not null,
    currency_code varchar(3) not null,
    amount_minor bigint not null,
    billing_interval varchar(16) null,
    trial_days integer not null default 0,
    sort_order integer not null default 0,
    is_default boolean not null default false,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_product_plans primary key (id),
    constraint uk_product_plans_plan_code unique (plan_code),
    constraint fk_product_plans_product_id
        foreign key (product_id)
        references products (id)
        on delete restrict
        on update restrict,
    constraint ck_product_plans_plan_status check (
        plan_status in ('draft', 'active', 'inactive', 'archived')
    ),
    constraint ck_product_plans_charge_type check (
        charge_type in ('one_time', 'subscription')
    ),
    constraint ck_product_plans_currency_code check (
        currency_code = upper(currency_code)
        and char_length(currency_code) = 3
    ),
    constraint ck_product_plans_amount_minor check (
        amount_minor >= 0
    ),
    constraint ck_product_plans_trial_days check (
        trial_days >= 0
    ),
    constraint ck_product_plans_billing_interval check (
        billing_interval in ('month', 'year')
        or billing_interval is null
    ),
    constraint ck_product_plans_subscription_fields check (
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

create index idx_products_product_status
    on products (product_status);

create index idx_product_plans_product_id
    on product_plans (product_id);

create index idx_product_plans_product_id_plan_status_sort_order
    on product_plans (product_id, plan_status, sort_order);

create unique index uk_product_plans_product_id_charge_type_default_active
    on product_plans (product_id, charge_type)
    where is_default = true and plan_status = 'active';
