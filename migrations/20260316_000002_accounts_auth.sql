create table kiro.users (
    id bigint generated always as identity,
    user_code varchar(64) not null,
    email varchar(320),
    email_normalized varchar(320),
    display_name varchar(128),
    avatar_url varchar(1024),
    locale varchar(16) not null default 'en-US',
    time_zone varchar(64) not null default 'UTC',
    status varchar(32) not null default 'pending',
    last_login_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_users primary key (id),
    check (status in ('pending', 'active', 'disabled', 'deleted'))
);

create unique index uk_users_user_code on kiro.users (user_code);
create unique index uk_users_email_norm on kiro.users (email_normalized)
    where email_normalized is not null;
create index idx_users_status on kiro.users (status);

create table kiro.user_identities (
    id bigint generated always as identity,
    identity_code varchar(64) not null,
    user_id bigint not null,
    provider varchar(32) not null,
    provider_user_id varchar(255) not null,
    provider_email varchar(320),
    provider_email_normalized varchar(320),
    profile_jsonb jsonb not null default '{}'::jsonb,
    last_authenticated_at timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_user_identities primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete cascade
        on update cascade,
    check (provider in ('google'))
);

create unique index uk_user_identities_code on kiro.user_identities (identity_code);
create unique index uk_user_identities_provider_uid
    on kiro.user_identities (provider, provider_user_id);
create index idx_user_identities_user_id on kiro.user_identities (user_id);
create index idx_user_identities_email_norm
    on kiro.user_identities (provider_email_normalized)
    where provider_email_normalized is not null;

create table kiro.auth_audit_logs (
    id bigint generated always as identity,
    audit_log_code varchar(64) not null,
    user_id bigint,
    provider varchar(32),
    event_type varchar(64) not null,
    result_status varchar(16) not null,
    ip_address inet,
    user_agent varchar(512),
    ua_hash char(64),
    trace_id varchar(64),
    failure_reason varchar(128),
    metadata_jsonb jsonb not null default '{}'::jsonb,
    occurred_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    constraint pk_auth_audit_logs primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete set null
        on update cascade,
    check (provider is null or provider in ('google')),
    check (result_status in ('success', 'failure', 'rejected'))
);

create unique index uk_auth_audit_logs_code on kiro.auth_audit_logs (audit_log_code);
create index idx_auth_audit_logs_user_time
    on kiro.auth_audit_logs (user_id, occurred_at desc);
create index idx_auth_audit_logs_event_time
    on kiro.auth_audit_logs (event_type, occurred_at desc);
