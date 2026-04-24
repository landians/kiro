# Kiro Postgres 数据表字典

## 1. 文档目的

本文档用于记录当前项目中已经通过 migration 定义的 PostgreSQL 数据表、表的业务作用，以及各字段的含义。

当前文档覆盖的 migration 来源如下：

- `kiro-api/migrations/20260323_000001_create_users.sql`
- `kiro-api/migrations/20260421_000002_create_products_and_billing_tables.sql`
- `kiro-api/migrations/20260424_000003_create_payment_orders.sql`
- `kiro-api/migrations/20260424_000004_create_payment_webhook_events.sql`
- `kiro-admin/migrations/20260404_000001_create_admin_users.sql`

如果后续新增、修改或删除表结构，需要同步更新本文档，保持文档与 migration 一致。

## 2. 表清单

当前已定义的 PostgreSQL 表如下：

- `users`
- `user_auth_identities`
- `products`
- `product_plans`
- `payment_orders`
- `payment_webhook_events`
- `admin_users`

## 3. 关系总览

- `users` 是普通用户主表。
- `user_auth_identities` 记录用户与第三方身份提供商之间的绑定关系。
- `products` 定义商品。
- `product_plans` 定义商品下的可售卖计划，支持一次性付费和订阅制。
- `payment_orders` 记录用户发起支付时生成的订单、价格快照以及支付结果状态。
- `payment_webhook_events` 记录支付渠道推送的 webhook 事件、去重状态和处理结果。
- `admin_users` 是后台管理员账号表，与普通用户表独立。

## 4. 表说明

### 4.1 `users`

用途：保存系统中的普通用户主体信息。

核心约束：

- 主键为数据库自增 `bigint`.
- `primary_email` 唯一。
- `account_status` 受检查约束控制，只允许 `active`、`frozen`、`banned`。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | 用户主键，内部关系标识。 |
| `primary_email` | `varchar(320)` | 用户主邮箱，可为空；如存在则全局唯一。 |
| `email_verified` | `boolean` | 主邮箱是否已验证。 |
| `display_name` | `varchar(255)` | 用户展示名。 |
| `avatar_url` | `text` | 用户头像地址。 |
| `account_status` | `varchar(32)` | 账号状态，`active` 表示正常，`frozen` 表示冻结，`banned` 表示封禁。 |
| `frozen_at` | `timestamptz` | 最近一次进入冻结状态的时间。 |
| `banned_at` | `timestamptz` | 最近一次进入封禁状态的时间。 |
| `last_login_at` | `timestamptz` | 最近一次登录时间。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

### 4.2 `user_auth_identities`

用途：保存用户与第三方登录身份之间的绑定关系，当前用于 Google 登录。

核心约束：

- `(provider, provider_user_id)` 唯一，防止同一第三方账号重复绑定。
- `(user_id, provider)` 唯一，防止同一用户在同一提供商下重复建立绑定。
- `user_id` 外键引用 `users.id`。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | 身份绑定主键。 |
| `user_id` | `bigint` | 关联的用户 ID，对应 `users.id`。 |
| `provider` | `varchar(32)` | 身份提供商，当前只允许 `google`。 |
| `provider_user_id` | `varchar(255)` | 第三方平台中的用户唯一标识。 |
| `provider_email` | `varchar(320)` | 第三方平台返回的邮箱。 |
| `provider_email_verified` | `boolean` | 第三方平台返回的邮箱是否已验证。 |
| `provider_display_name` | `varchar(255)` | 第三方平台返回的展示名。 |
| `provider_avatar_url` | `text` | 第三方平台返回的头像地址。 |
| `last_login_at` | `timestamptz` | 最近一次通过该身份登录的时间。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

### 4.3 `products`

用途：定义商品本体，即“卖的是什么”。

补充说明：

- 如需查看 `products` / `product_plans` 与 Stripe、Creem 商品 / 价格模型的映射关系，可参考 `docs/product-catalog-provider-mapping.md`。

核心约束：

- `product_code` 唯一，作为商品的稳定业务编码。
- `product_status` 受检查约束控制。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | 商品主键。 |
| `product_code` | `varchar(64)` | 商品稳定编码，例如 `pro`、`team`。 |
| `product_name` | `varchar(128)` | 商品名称。 |
| `product_description` | `text` | 商品描述。 |
| `product_image_url` | `text` | 商品展示图片地址，通常用于商品列表卡片、详情页封面等主图展示。 |
| `product_status` | `varchar(32)` | 商品状态，允许 `draft`、`active`、`inactive`、`archived`。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

### 4.4 `product_plans`

用途：定义商品的具体售卖计划，即“怎么卖、卖多少钱”，支持一次性付费与订阅制。

补充说明：

- 如需查看 `products` / `product_plans` 与 Stripe、Creem 商品 / 价格模型的映射关系，可参考 `docs/product-catalog-provider-mapping.md`。

核心约束：

- `product_id` 外键引用 `products.id`。
- `plan_code` 全局唯一。
- `charge_type` 区分 `one_time` 与 `subscription`。
- 检查约束用于保证订阅字段和一次性付费字段组合合法。
- 存在部分唯一索引，限制同一商品同一收费类型只能有一个默认激活计划。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | 计划主键。 |
| `product_id` | `bigint` | 所属商品 ID，对应 `products.id`。 |
| `plan_code` | `varchar(64)` | 计划稳定编码，例如 `pro_monthly`。 |
| `plan_name` | `varchar(128)` | 计划名称。 |
| `plan_status` | `varchar(32)` | 计划状态，允许 `draft`、`active`、`inactive`、`archived`。 |
| `charge_type` | `varchar(32)` | 收费方式，允许 `one_time` 或 `subscription`。 |
| `currency_code` | `varchar(3)` | 三位大写货币代码，例如 `CNY`、`USD`。 |
| `amount_minor` | `bigint` | 以最小货币单位表示的金额，例如分、cent。 |
| `billing_interval` | `varchar(16)` | 订阅计费周期单位，允许 `month`、`year`；一次性付费为空。 |
| `trial_days` | `integer` | 试用天数，仅订阅计划使用。 |
| `sort_order` | `integer` | 同一商品下计划排序值。 |
| `is_default` | `boolean` | 是否为默认推荐计划。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

### 4.5 `payment_orders`

用途：记录用户发起购买后生成的支付订单，承担下单时价格快照、支付渠道绑定、支付状态流转和后续 webhook 回写落点。

核心约束：

- `order_no` 全局唯一，作为对外可暴露的稳定订单号。
- `user_id`、`product_id`、`product_plan_id` 分别外键引用 `users.id`、`products.id`、`product_plans.id`。
- `payment_provider` 当前只允许 `stripe`、`creem`。
- `order_status` 当前允许 `pending`、`paid`、`failed`、`canceled`、`refunded`。
- 下单时保存商品、计划、金额、币种、收费方式等价格快照，避免历史订单受后续改价影响。
- 通过部分唯一索引约束同一 provider 下的 `provider_checkout_session_id` 与 `provider_payment_id` 不重复。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | 订单主键。 |
| `order_no` | `varchar(64)` | 对外稳定订单号。 |
| `user_id` | `bigint` | 下单用户 ID，对应 `users.id`。 |
| `product_id` | `bigint` | 下单时关联的商品 ID，对应 `products.id`。 |
| `product_plan_id` | `bigint` | 下单时关联的商品计划 ID，对应 `product_plans.id`。 |
| `payment_provider` | `varchar(32)` | 支付渠道，当前允许 `stripe`、`creem`。 |
| `order_status` | `varchar(32)` | 订单状态，当前允许 `pending`、`paid`、`failed`、`canceled`、`refunded`。 |
| `provider_checkout_session_id` | `varchar(255)` | 第三方支付渠道返回的 checkout/session 标识。 |
| `provider_payment_id` | `varchar(255)` | 第三方支付渠道的最终支付标识，例如 payment intent / payment id。 |
| `provider_customer_id` | `varchar(255)` | 第三方支付渠道中的客户标识。 |
| `product_code` | `varchar(64)` | 下单时的商品编码快照。 |
| `product_name` | `varchar(128)` | 下单时的商品名称快照。 |
| `product_image_url` | `text` | 下单时的商品图片地址快照。 |
| `plan_code` | `varchar(64)` | 下单时的计划编码快照。 |
| `plan_name` | `varchar(128)` | 下单时的计划名称快照。 |
| `charge_type` | `varchar(32)` | 收费方式快照，允许 `one_time` 或 `subscription`。 |
| `currency_code` | `varchar(3)` | 订单币种快照，三位大写货币代码。 |
| `amount_minor` | `bigint` | 订单金额快照，按最小货币单位存储。 |
| `billing_interval` | `varchar(16)` | 订阅周期快照，允许 `month`、`year`；一次性订单为空。 |
| `trial_days` | `integer` | 试用天数快照，仅订阅订单使用。 |
| `failure_code` | `varchar(64)` | 支付失败或取消时的业务/渠道错误码。 |
| `failure_message` | `text` | 支付失败或取消时的补充说明。 |
| `expires_at` | `timestamptz` | 订单或 checkout session 过期时间。 |
| `paid_at` | `timestamptz` | 订单确认支付成功时间。 |
| `failed_at` | `timestamptz` | 订单确认支付失败时间。 |
| `canceled_at` | `timestamptz` | 订单被取消时间。 |
| `refunded_at` | `timestamptz` | 订单确认退款完成时间。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

### 4.6 `payment_webhook_events`

用途：记录支付渠道推送的 webhook 事件，承担事件去重、处理状态跟踪、失败重试和问题排查的“收件箱”职责。

核心约束：

- `(payment_provider, provider_event_id)` 唯一，保证同一渠道事件只会被落库一次。
- `payment_order_id` 可为空，因为部分 webhook 在落库时可能暂时还无法关联到本地订单。
- `processing_status` 当前允许 `received`、`processing`、`processed`、`failed`、`ignored`。
- `payload` 使用 `jsonb` 保存渠道原始事件内容，适合作为不稳定的外部扩展数据。
- 通过处理状态索引与事件类型索引支持异步消费、重试与排查。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | webhook 事件记录主键。 |
| `payment_order_id` | `bigint` | 关联的本地支付订单 ID，对应 `payment_orders.id`；无法立即关联时可为空。 |
| `payment_provider` | `varchar(32)` | 事件来源支付渠道，当前允许 `stripe`、`creem`。 |
| `provider_event_id` | `varchar(255)` | 渠道侧 webhook 事件唯一标识。 |
| `event_type` | `varchar(128)` | 渠道侧事件类型，例如 `checkout.session.completed`。 |
| `event_object_id` | `varchar(255)` | 事件中主资源对象的标识，例如 session / payment intent / invoice id。 |
| `processing_status` | `varchar(32)` | 处理状态，当前允许 `received`、`processing`、`processed`、`failed`、`ignored`。 |
| `retry_count` | `integer` | 当前事件已尝试处理的次数。 |
| `payload` | `jsonb` | 渠道原始 webhook 事件负载。 |
| `error_message` | `text` | 最近一次处理失败时的错误信息。 |
| `received_at` | `timestamptz` | 系统收到该 webhook 的时间。 |
| `processed_at` | `timestamptz` | 成功处理完成时间。 |
| `last_error_at` | `timestamptz` | 最近一次处理失败时间。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

### 4.7 `admin_users`

用途：保存后台管理员账号信息，与普通用户体系独立。

核心约束：

- `email` 唯一。
- `account_status` 只允许 `active` 和 `frozen`。

字段说明：

| 字段名 | 类型 | 含义 |
| --- | --- | --- |
| `id` | `bigint` | 管理员主键。 |
| `email` | `varchar(320)` | 管理员登录邮箱，唯一。 |
| `password_hash` | `varchar(255)` | 密码哈希值。 |
| `display_name` | `varchar(255)` | 管理员展示名。 |
| `account_status` | `varchar(32)` | 管理员账号状态，允许 `active`、`frozen`。 |
| `last_login_at` | `timestamptz` | 最近一次后台登录时间。 |
| `created_at` | `timestamptz` | 创建时间。 |
| `updated_at` | `timestamptz` | 更新时间。 |

## 5. 使用建议

- 新增表或字段时，先更新 migration，再同步更新本文档。
- 如果字段语义发生变化，不只修改字段名，还应同步修改“字段含义”描述。
- `jsonb` 字段只用于扩展属性，不应把可建模的核心关系字段长期藏在 `metadata` 中。
- 业务查询和开发联调时，优先查本文档确认字段语义，再查 migration 细节。
