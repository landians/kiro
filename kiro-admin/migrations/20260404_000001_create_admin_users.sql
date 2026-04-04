create table admin_users (
    id bigint generated always as identity,
    email varchar(320) not null,
    password_hash varchar(255) not null,
    display_name varchar(255) null,
    account_status varchar(32) not null default 'active',
    last_login_at timestamptz null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_admin_users primary key (id),
    constraint uk_admin_users_email unique (email),
    constraint ck_admin_users_account_status check (
        account_status in ('active', 'frozen')
    )
);
