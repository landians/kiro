create table kiro.supported_locales (
    id bigint generated always as identity,
    locale_code varchar(16) not null,
    display_name varchar(64) not null,
    is_default boolean not null default false,
    is_active boolean not null default true,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_supported_locales primary key (id)
);

create unique index uk_supported_locales_code on kiro.supported_locales (locale_code);
create unique index uk_supported_locales_default on kiro.supported_locales (is_default)
    where is_default;

create table kiro.products (
    id bigint generated always as identity,
    product_code varchar(64) not null,
    product_type varchar(32) not null default 'subscription',
    default_locale varchar(16) not null default 'en-US',
    status varchar(32) not null default 'draft',
    visibility_status varchar(32) not null default 'hidden',
    sort_order integer not null default 0,
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_products primary key (id),
    check (product_type in ('subscription', 'one_time')),
    check (status in ('draft', 'active', 'archived')),
    check (visibility_status in ('hidden', 'public'))
);

create unique index uk_products_code on kiro.products (product_code);
create index idx_products_status_visibility
    on kiro.products (status, visibility_status, sort_order, id);

create table kiro.product_translations (
    id bigint generated always as identity,
    product_id bigint not null,
    locale varchar(16) not null,
    display_name varchar(128) not null,
    short_description varchar(255),
    description text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_product_translations primary key (id),
    foreign key (product_id) references kiro.products (id)
        on delete cascade
        on update cascade
);

create unique index uk_product_translations_locale
    on kiro.product_translations (product_id, locale);
create index idx_product_translations_locale on kiro.product_translations (locale);

create table kiro.product_plans (
    id bigint generated always as identity,
    plan_code varchar(64) not null,
    product_id bigint not null,
    billing_interval varchar(32) not null,
    interval_count integer not null default 1,
    trial_days integer,
    status varchar(32) not null default 'draft',
    is_default boolean not null default false,
    sort_order integer not null default 0,
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_product_plans primary key (id),
    foreign key (product_id) references kiro.products (id)
        on delete cascade
        on update cascade,
    check (billing_interval in ('one_time', 'day', 'week', 'month', 'year')),
    check (interval_count > 0),
    check (trial_days is null or trial_days >= 0),
    check (status in ('draft', 'active', 'inactive', 'archived'))
);

create unique index uk_product_plans_code on kiro.product_plans (plan_code);
create unique index uk_product_plans_default on kiro.product_plans (product_id)
    where is_default;
create index idx_product_plans_product_status
    on kiro.product_plans (product_id, status, sort_order, id);

create table kiro.product_plan_translations (
    id bigint generated always as identity,
    plan_id bigint not null,
    locale varchar(16) not null,
    display_name varchar(128) not null,
    description text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_product_plan_translations primary key (id),
    foreign key (plan_id) references kiro.product_plans (id)
        on delete cascade
        on update cascade
);

create unique index uk_product_plan_translations_locale
    on kiro.product_plan_translations (plan_id, locale);
create index idx_product_plan_translations_locale
    on kiro.product_plan_translations (locale);

create table kiro.product_prices (
    id bigint generated always as identity,
    price_code varchar(64) not null,
    plan_id bigint not null,
    provider varchar(32) not null,
    provider_price_id varchar(128),
    currency_code char(3) not null,
    amount_minor bigint not null,
    status varchar(32) not null default 'draft',
    effective_start_at timestamptz,
    effective_end_at timestamptz,
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_product_prices primary key (id),
    foreign key (plan_id) references kiro.product_plans (id)
        on delete cascade
        on update cascade,
    check (provider in ('stripe', 'creem')),
    check (amount_minor >= 0),
    check (status in ('draft', 'active', 'inactive', 'archived')),
    check (
        effective_end_at is null
        or effective_start_at is null
        or effective_end_at > effective_start_at
    )
);

create unique index uk_product_prices_code on kiro.product_prices (price_code);
create unique index uk_product_prices_provider_pid
    on kiro.product_prices (provider, provider_price_id)
    where provider_price_id is not null;
create index idx_product_prices_plan_status
    on kiro.product_prices (plan_id, status, currency_code, id);

create table kiro.entitlements (
    id bigint generated always as identity,
    entitlement_code varchar(64) not null,
    entitlement_key varchar(128) not null,
    entitlement_type varchar(32) not null default 'feature',
    status varchar(32) not null default 'active',
    sort_order integer not null default 0,
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_entitlements primary key (id),
    check (entitlement_type in ('feature', 'quota', 'download', 'role')),
    check (status in ('active', 'inactive', 'archived'))
);

create unique index uk_entitlements_code on kiro.entitlements (entitlement_code);
create unique index uk_entitlements_key on kiro.entitlements (entitlement_key);
create index idx_entitlements_status on kiro.entitlements (status, sort_order, id);

create table kiro.entitlement_translations (
    id bigint generated always as identity,
    entitlement_id bigint not null,
    locale varchar(16) not null,
    display_name varchar(128) not null,
    description text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_entitlement_translations primary key (id),
    foreign key (entitlement_id) references kiro.entitlements (id)
        on delete cascade
        on update cascade
);

create unique index uk_entitlement_translations_locale
    on kiro.entitlement_translations (entitlement_id, locale);
create index idx_entitlement_translations_locale
    on kiro.entitlement_translations (locale);

create table kiro.product_plan_entitlements (
    id bigint generated always as identity,
    plan_id bigint not null,
    entitlement_id bigint not null,
    grant_type varchar(32) not null default 'standard',
    quota_value bigint,
    quota_period varchar(32),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_product_plan_entitlements primary key (id),
    foreign key (plan_id) references kiro.product_plans (id)
        on delete cascade
        on update cascade,
    foreign key (entitlement_id) references kiro.entitlements (id)
        on delete restrict
        on update cascade,
    check (grant_type in ('standard', 'trial', 'bonus')),
    check (quota_value is null or quota_value > 0),
    check (
        quota_period is null
        or quota_period in ('day', 'week', 'month', 'year', 'lifetime')
    )
);

create unique index uk_plan_entitlements_unique
    on kiro.product_plan_entitlements (plan_id, entitlement_id, grant_type);
create index idx_plan_entitlements_entitlement_id
    on kiro.product_plan_entitlements (entitlement_id);
