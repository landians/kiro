# Kiro Postgres 数据表字典

## 1. 文档目的

本文档用于记录当前项目中已经通过 migration 定义的 PostgreSQL 数据表、表的业务作用，以及各字段的含义。

当前文档覆盖的 migration 来源如下：

- `kiro-api/migrations/20260323_000001_create_users.sql`
- `kiro-api/migrations/20260421_000002_create_products_and_billing_tables.sql`
- `kiro-admin/migrations/20260404_000001_create_admin_users.sql`

如果后续新增、修改或删除表结构，需要同步更新本文档，保持文档与 migration 一致。

## 2. 表清单

当前已定义的 PostgreSQL 表如下：

- `users`
- `user_auth_identities`
- `products`
- `product_plans`
- `admin_users`

## 3. 关系总览

- `users` 是普通用户主表。
- `user_auth_identities` 记录用户与第三方身份提供商之间的绑定关系。
- `products` 定义商品。
- `product_plans` 定义商品下的可售卖计划，支持一次性付费和订阅制。
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

### 4.5 `admin_users`

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
