create table kiro.email_templates (
    id bigint generated always as identity,
    template_code varchar(64) not null,
    template_purpose varchar(64) not null,
    default_locale varchar(16) not null default 'en-US',
    status varchar(32) not null default 'draft',
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_email_templates primary key (id),
    check (status in ('draft', 'active', 'archived'))
);

create unique index uk_email_templates_code on kiro.email_templates (template_code);
create index idx_email_templates_status on kiro.email_templates (status, template_purpose);

create table kiro.email_template_translations (
    id bigint generated always as identity,
    email_template_id bigint not null,
    locale varchar(16) not null,
    subject_template varchar(255) not null,
    html_body_template text not null,
    text_body_template text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_email_template_translations primary key (id),
    foreign key (email_template_id) references kiro.email_templates (id)
        on delete cascade
        on update cascade
);

create unique index uk_email_template_translations_locale
    on kiro.email_template_translations (email_template_id, locale);

create table kiro.email_messages (
    id bigint generated always as identity,
    email_message_code varchar(64) not null,
    user_id bigint,
    email_template_id bigint,
    locale varchar(16) not null,
    recipient_email varchar(320) not null,
    subject_snapshot varchar(255) not null,
    delivery_status varchar(32) not null default 'pending',
    provider varchar(64),
    provider_message_id varchar(128),
    idempotency_key varchar(128),
    scheduled_at timestamptz,
    sent_at timestamptz,
    failed_at timestamptz,
    failure_code varchar(64),
    failure_message varchar(256),
    payload_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_email_messages primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete set null
        on update cascade,
    foreign key (email_template_id) references kiro.email_templates (id)
        on delete set null
        on update cascade,
    check (delivery_status in ('pending', 'processing', 'sent', 'failed', 'canceled'))
);

create unique index uk_email_messages_code on kiro.email_messages (email_message_code);
create unique index uk_email_messages_idem on kiro.email_messages (idempotency_key)
    where idempotency_key is not null;
create unique index uk_email_messages_provider_mid
    on kiro.email_messages (provider, provider_message_id)
    where provider_message_id is not null;
create index idx_email_messages_user_status
    on kiro.email_messages (user_id, delivery_status, created_at desc);
create index idx_email_messages_status_time
    on kiro.email_messages (delivery_status, created_at desc);

create table kiro.notification_templates (
    id bigint generated always as identity,
    template_code varchar(64) not null,
    category varchar(32) not null,
    default_locale varchar(16) not null default 'en-US',
    status varchar(32) not null default 'draft',
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_notification_templates primary key (id),
    check (status in ('draft', 'active', 'archived'))
);

create unique index uk_notification_templates_code
    on kiro.notification_templates (template_code);
create index idx_notification_templates_status
    on kiro.notification_templates (status, category);

create table kiro.notification_template_translations (
    id bigint generated always as identity,
    notification_template_id bigint not null,
    locale varchar(16) not null,
    title_template varchar(255) not null,
    body_template text not null,
    action_label_template varchar(64),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_notification_template_translations primary key (id),
    foreign key (notification_template_id) references kiro.notification_templates (id)
        on delete cascade
        on update cascade
);

create unique index uk_notification_template_translations_locale
    on kiro.notification_template_translations (notification_template_id, locale);

create table kiro.notifications (
    id bigint generated always as identity,
    notification_code varchar(64) not null,
    user_id bigint not null,
    notification_template_id bigint,
    locale varchar(16) not null,
    category varchar(32) not null,
    title_snapshot varchar(255) not null,
    body_snapshot text not null,
    action_url varchar(1024),
    notification_status varchar(32) not null default 'unread',
    deduplication_key varchar(128),
    delivered_at timestamptz,
    read_at timestamptz,
    payload_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_notifications primary key (id),
    foreign key (user_id) references kiro.users (id)
        on delete cascade
        on update cascade,
    foreign key (notification_template_id) references kiro.notification_templates (id)
        on delete set null
        on update cascade,
    check (notification_status in ('unread', 'read', 'archived'))
);

create unique index uk_notifications_code on kiro.notifications (notification_code);
create unique index uk_notifications_user_dedup
    on kiro.notifications (user_id, deduplication_key)
    where deduplication_key is not null;
create index idx_notifications_user_status_time
    on kiro.notifications (user_id, notification_status, created_at desc);
create index idx_notifications_unread
    on kiro.notifications (user_id, created_at desc)
    where notification_status = 'unread';

create table kiro.invitations (
    id bigint generated always as identity,
    invitation_code varchar(64) not null,
    inviter_user_id bigint not null,
    invitee_user_id bigint,
    invitee_email varchar(320),
    invitee_email_normalized varchar(320),
    token_hash varchar(128) not null,
    invitation_status varchar(32) not null default 'pending',
    reward_status varchar(32) not null default 'pending',
    expires_at timestamptz not null,
    accepted_at timestamptz,
    revoked_at timestamptz,
    accepted_trace_id varchar(64),
    metadata_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_invitations primary key (id),
    foreign key (inviter_user_id) references kiro.users (id)
        on delete restrict
        on update cascade,
    foreign key (invitee_user_id) references kiro.users (id)
        on delete set null
        on update cascade,
    check (invitee_user_id is not null or invitee_email_normalized is not null),
    check (invitation_status in ('pending', 'accepted', 'expired', 'revoked')),
    check (reward_status in ('pending', 'qualified', 'granted', 'rejected'))
);

create unique index uk_invitations_code on kiro.invitations (invitation_code);
create unique index uk_invitations_token_hash on kiro.invitations (token_hash);
create index idx_invitations_inviter_status_time
    on kiro.invitations (inviter_user_id, invitation_status, created_at desc);
create index idx_invitations_invitee_user
    on kiro.invitations (invitee_user_id)
    where invitee_user_id is not null;
create index idx_invitations_invitee_email
    on kiro.invitations (invitee_email_normalized)
    where invitee_email_normalized is not null;

create table kiro.invitation_rewards (
    id bigint generated always as identity,
    reward_code varchar(64) not null,
    invitation_id bigint not null,
    rewarded_user_id bigint not null,
    reward_type varchar(32) not null,
    reward_status varchar(32) not null default 'pending',
    reward_value_minor bigint,
    reward_currency_code char(3),
    granted_at timestamptz,
    reversed_at timestamptz,
    payload_jsonb jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_invitation_rewards primary key (id),
    foreign key (invitation_id) references kiro.invitations (id)
        on delete cascade
        on update cascade,
    foreign key (rewarded_user_id) references kiro.users (id)
        on delete restrict
        on update cascade,
    check (reward_type in ('credit', 'discount', 'feature', 'coupon')),
    check (reward_status in ('pending', 'granted', 'failed', 'reversed')),
    check (reward_value_minor is null or reward_value_minor >= 0)
);

create unique index uk_invitation_rewards_code on kiro.invitation_rewards (reward_code);
create unique index uk_invitation_rewards_unique
    on kiro.invitation_rewards (invitation_id, rewarded_user_id, reward_type);
create index idx_invitation_rewards_user_status
    on kiro.invitation_rewards (rewarded_user_id, reward_status, created_at desc);
