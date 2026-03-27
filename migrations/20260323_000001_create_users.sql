create table users (
    id bigint generated always as identity,
    primary_email varchar(320) null,
    email_verified boolean not null default false,
    display_name varchar(255) null,
    avatar_url text null,
    account_status varchar(32) not null default 'active',
    frozen_at timestamptz null,
    banned_at timestamptz null,
    last_login_at timestamptz null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_users primary key (id),
    constraint uk_users_primary_email unique (primary_email),
    constraint ck_users_account_status check (
        account_status in ('active', 'frozen', 'banned')
    )
);

create table user_auth_identities (
    id bigint generated always as identity,
    user_id bigint not null,
    provider varchar(32) not null,
    provider_user_id varchar(255) not null,
    provider_email varchar(320) null,
    provider_email_verified boolean not null default false,
    provider_display_name varchar(255) null,
    provider_avatar_url text null,
    provider_profile jsonb not null default '{}'::jsonb,
    last_login_at timestamptz null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_uai primary key (id),
    constraint uk_uai_provider_user_id unique (
        provider,
        provider_user_id
    ),
    constraint uk_uai_user_id_provider unique (user_id, provider),
    constraint fk_uai_user_id
        foreign key (user_id)
        references users (id)
        on delete restrict
        on update restrict,
    constraint ck_uai_provider check (
        provider in ('google')
    )
);
