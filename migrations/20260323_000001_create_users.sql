create schema if not exists kiro;

create table kiro.users (
    id bigint generated always as identity,
    provider varchar(32) not null,
    provider_user_id varchar(255) not null,
    email varchar(320) null,
    email_verified boolean not null default false,
    display_name varchar(255) null,
    avatar_url text null,
    account_status varchar(32) not null default 'active',
    frozen_at timestamptz null,
    banned_at timestamptz null,
    last_login_at timestamptz not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_users primary key (id),
    constraint uk_users_provider_provider_user_id unique (provider, provider_user_id),
    constraint ck_users_account_status check (
        account_status in ('active', 'frozen', 'banned')
    )
);

create index idx_users_email
    on kiro.users (email);
