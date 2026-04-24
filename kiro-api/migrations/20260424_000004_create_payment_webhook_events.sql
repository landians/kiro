create table payment_webhook_events (
    id bigint generated always as identity,
    payment_order_id bigint null,
    payment_provider varchar(32) not null,
    provider_event_id varchar(255) not null,
    event_type varchar(128) not null,
    event_object_id varchar(255) null,
    processing_status varchar(32) not null default 'received',
    retry_count integer not null default 0,
    payload jsonb not null,
    error_message text null,
    received_at timestamptz not null default now(),
    processed_at timestamptz null,
    last_error_at timestamptz null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    constraint pk_payment_webhook_events primary key (id),
    constraint fk_payment_webhook_events_payment_order_id
        foreign key (payment_order_id)
        references payment_orders (id)
        on delete set null
        on update restrict,
    constraint ck_payment_webhook_events_payment_provider check (
        payment_provider in ('stripe', 'creem')
    ),
    constraint ck_payment_webhook_events_processing_status check (
        processing_status in ('received', 'processing', 'processed', 'failed', 'ignored')
    ),
    constraint ck_payment_webhook_events_retry_count check (
        retry_count >= 0
    )
);

create unique index uk_payment_webhook_events_provider_event_id
    on payment_webhook_events (payment_provider, provider_event_id);

create index idx_payment_webhook_events_payment_order_id_received_at
    on payment_webhook_events (payment_order_id, received_at desc);

create index idx_payment_webhook_events_processing_status_received_at
    on payment_webhook_events (processing_status, received_at asc);

create index idx_payment_webhook_events_event_type_received_at
    on payment_webhook_events (payment_provider, event_type, received_at desc);

create index idx_payment_webhook_events_payload_gin
    on payment_webhook_events
    using gin (payload);
