create table kiro.admin_users (
    id bigint generated always as identity,
    admin_user_code varchar(64) not null,
    user_id bigint not null,
    admin_status varchar(32) not null default 'active',
    permission_scope varchar(32) not null default 'all',
    granted_by_user_id bigint,
    revoked_by_user_id bigint,
    granted_at timestamptz not null default now(),
    revoked_at timestamptz,
    last_login_at timestamptz,
    notes varchar(255),
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_admin_users primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete restrict
        on update cascade,
    foreign key (granted_by_user_id) references kiro.users (id)
        on delete set null
        on update cascade,
    foreign key (revoked_by_user_id) references kiro.users (id)
        on delete set null
        on update cascade,
    check (admin_status in ('active', 'inactive', 'revoked')),
    check (permission_scope in ('all'))
);

create unique index uk_admin_users_code on kiro.admin_users (admin_user_code);
create unique index uk_admin_users_user_id on kiro.admin_users (user_id);
create index idx_admin_users_status_time
    on kiro.admin_users (admin_status, created_at desc);

create table kiro.admin_operation_logs (
    id bigint generated always as identity,
    admin_operation_log_code varchar(64) not null,
    admin_user_id bigint not null,
    module_name varchar(64) not null,
    action_name varchar(64) not null,
    target_type varchar(64),
    target_id varchar(128),
    result_status varchar(16) not null,
    trace_id varchar(64),
    ip_address inet,
    user_agent varchar(512),
    request_jsonb jsonb not null default '{}'::jsonb,
    response_jsonb jsonb not null default '{}'::jsonb,
    error_message varchar(256),
    occurred_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    constraint pk_admin_operation_logs primary key (id),
    foreign key (admin_user_id) references kiro.admin_users (id)
        on delete restrict
        on update cascade,
    check (result_status in ('success', 'failure', 'rejected'))
);

create unique index uk_admin_operation_logs_code
    on kiro.admin_operation_logs (admin_operation_log_code);
create index idx_admin_operation_logs_admin_time
    on kiro.admin_operation_logs (admin_user_id, occurred_at desc);
create index idx_admin_operation_logs_module_time
    on kiro.admin_operation_logs (module_name, occurred_at desc);
