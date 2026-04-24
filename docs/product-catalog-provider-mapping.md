# Kiro 商品目录与 Stripe / Creem 商品 / 价格映射关系

## 1. 文档目的

本文档用于说明当前 Kiro 项目中 `products`、`product_plans` 两张表的字段语义，以及它们与 Stripe、Creem 商品 / 价格模型之间的映射关系。

- 本文档描述的是“当前内部表结构”和“对外支付平台对象模型”之间的映射。
- Stripe 与 Creem 的对象模型并不一致，因此映射方式也不同。
- 以下关于 Stripe / Creem 的映射关系，是基于官方文档对象模型做出的集成设计建议，不代表当前仓库已经落地了对应字段、同步任务或 provider ID 持久化逻辑。

## 2. 当前内部模型

当前 Kiro 采用两层商品目录模型：

- `products`：定义“卖的是什么”，承担展示层商品主体角色。
- `product_plans`：定义“怎么卖”，承担价格、收费方式、周期等售卖规则角色。

关系是：

- 一条 `products` 可以对应多条 `product_plans`
- 一条 `product_plans` 必须归属于一条 `products`

这套模型更接近“目录商品 + 售卖计划”的内部业务建模，而不是简单复制某个支付平台的对象结构。

## 3. 当前表字段

### 3.1 `products`

| 字段 | 含义 | 当前职责 |
| --- | --- | --- |
| `id` | 内部商品主键 | 内部关系关联 |
| `product_code` | 商品稳定编码 | 对内/对外稳定业务标识 |
| `product_name` | 商品名称 | 展示名称 |
| `product_description` | 商品描述 | 展示说明 |
| `product_image_url` | 商品图片地址 | 商品主图展示 |
| `product_status` | `draft/active/inactive/archived` | 目录上架状态 |
| `created_at` / `updated_at` | 时间戳 | 审计与排序 |

### 3.2 `product_plans`

| 字段 | 含义 | 当前职责 |
| --- | --- | --- |
| `id` | 内部计划主键 | 内部关系关联 |
| `product_id` | 关联 `products.id` | 归属商品 |
| `plan_code` | 计划稳定编码 | 售卖计划业务标识 |
| `plan_name` | 计划名称 | 套餐展示名称 |
| `plan_status` | `draft/active/inactive/archived` | 计划上架状态 |
| `charge_type` | `one_time/subscription` | 一次性 / 订阅 |
| `currency_code` | 货币代码 | 定价币种 |
| `amount_minor` | 最小货币单位金额 | 定价金额 |
| `billing_interval` | `month/year` 或空 | 订阅周期 |
| `trial_days` | 试用天数 | 订阅试用 |
| `sort_order` | 排序值 | 前端排序/推荐 |
| `is_default` | 是否默认计划 | 默认推荐计划 |
| `created_at` / `updated_at` | 时间戳 | 审计与排序 |

## 4. Stripe 映射

## 4.1 Stripe 官方对象模型

根据 Stripe 官方文档：

- `Product` 定义卖的是什么。
- `Price` 定义卖多少钱、按什么周期收费。
- 一个 `Product` 可以挂多条 `Price`。

这与 Kiro 当前的 `products -> product_plans` 模型是天然对齐的。

## 4.2 Kiro 到 Stripe 的主映射关系

- `products` 一般映射为 Stripe `Product`
- `product_plans` 一般映射为 Stripe `Price`

也就是说：

- 一条内部商品
  -> 一条 Stripe `Product`
- 一条内部计划
  -> 该 Stripe `Product` 下的一条 Stripe `Price`

## 4.3 `products` -> Stripe `Product` 字段映射

| Kiro 字段 | Stripe 对象字段 | 说明 |
| --- | --- | --- |
| `product_code` | `product.id` 或 `metadata.product_code` | Stripe 允许自定义 Product ID；如果不自定义，则至少应写入 metadata 保持稳定映射 |
| `product_name` | `product.name` | 直接映射 |
| `product_description` | `product.description` | 直接映射 |
| `product_image_url` | `product.images[0]` | Stripe Product 支持图片数组，当前内部只有一张主图 |
| `product_status` | `product.active` | `active` 可映射为 `true`；其余内部状态一般映射为 `false` 或通过业务同步层控制 |

## 4.4 `product_plans` -> Stripe `Price` 字段映射

| Kiro 字段 | Stripe 对象字段 | 说明 |
| --- | --- | --- |
| `plan_code` | `price.lookup_key` 或 `metadata.plan_code` | Stripe `Price` 支持 `lookup_key`，很适合承载稳定计划编码 |
| `plan_name` | `price.nickname` 或 `metadata.plan_name` | Stripe `Price.nickname` 面向内部简述，不是客户主展示名称 |
| `currency_code` | `price.currency` | Stripe 要求小写 ISO 货币代码；内部是大写，落地时需要转换 |
| `amount_minor` | `price.unit_amount` | 直接映射，单位都是最小货币单位 |
| `charge_type = one_time` | `price.type = one_time` | 一次性价格 |
| `charge_type = subscription` | `price.type = recurring` | 订阅价格 |
| `billing_interval` | `price.recurring.interval` | 当前内部支持 `month` / `year` |
| `trial_days` | 订阅创建参数，例如 Checkout Session / Subscription 创建时的 trial 配置 | Stripe `Price` 主要描述价格本身，试用期通常不直接挂在 `Price` 对象上，更适合在创建订阅或结账会话时注入 |
| `plan_status` | `price.active` | `active` 可映射为 `true`，其余一般映射为 `false` |

## 4.5 Stripe 映射结论

Stripe 与 Kiro 的当前模型高度一致，推荐策略是：

- `products` 作为 Stripe `Product`
- `product_plans` 作为 Stripe `Price`
- `product_code` / `plan_code` 作为稳定业务编码，优先写入 Stripe 的可检索标识字段

这也是当前 Kiro 模型更偏向 Stripe 风格的主要原因。

## 5. Creem 映射

## 5.1 Creem 官方对象模型

根据当前 Creem `Create Product` / `Get Product` 文档，Creem 的 `Product` 对象本身同时包含：

- `name`
- `description`
- `image_url`
- `price`
- `currency`
- `billing_type`
- `billing_period`

也就是说，Creem 当前公开文档中的商品对象本身就同时承载了“商品展示信息”和“价格 / 计费方式”。

这和 Stripe 的 `Product` / `Price` 分离模型明显不同。

## 5.2 Kiro 到 Creem 的主映射关系

对于 Creem，更合理的映射不是：

- `products` -> Creem `Product`
- `product_plans` -> Creem “某个独立价格对象”

因为 Creem 并没有像 Stripe 那样分离的 `Price` 目录对象。

更合适的映射是：

- 一条 `products` + 一条 `product_plans`
  -> 一条 Creem `Product`

也就是说：

- `Kiro product` 负责“展示主体”
- `Kiro plan` 负责“定价与收费方式”
- 最终同步到 Creem 时，需要把两者合并成一个 Creem Product

## 5.3 `products + product_plans` -> Creem `Product` 字段映射

| Kiro 字段 | Creem 对象字段 | 说明 |
| --- | --- | --- |
| `products.product_name` | `product.name` | 直接映射 |
| `products.product_description` | `product.description` | 直接映射 |
| `products.product_image_url` | `product.image_url` | 直接映射 |
| `product_plans.amount_minor` | `product.price` | Creem `price` 为分 / cents 语义，和内部 `amount_minor` 可以直接对齐 |
| `product_plans.currency_code` | `product.currency` | Creem 使用大写货币代码，和内部一致 |
| `product_plans.charge_type = one_time` | `product.billing_type = onetime` | Creem 使用 `onetime`，内部使用 `one_time`，需要转换 |
| `product_plans.charge_type = subscription` | `product.billing_type = recurring` | 订阅计费 |
| `product_plans.billing_interval = month` | `product.billing_period = every-month` | 周期值需要转换 |
| `product_plans.billing_interval = year` | `product.billing_period = every-year` | 周期值需要转换 |
| `product_plans.plan_status` / `products.product_status` | `product.status` 或同步层下架策略 | 当前可以确认 Creem Product 响应里存在 `status` 字段，但内部状态是否与其逐值一一映射，应由同步层统一约定 |

## 5.4 一个内部商品多个计划时如何映射到 Creem

因为 Creem 的 Product 自带价格与计费周期，所以如果内部一个商品有多条计划，例如：

- `pro_monthly`
- `pro_yearly`
- `pro_onetime`

那么在 Creem 里通常需要表现为多条 Product，而不是一条 Product 下挂多条 Price。

因此更准确的映射是：

- 一条 `products`
- 多条 `product_plans`
- 在 Creem 中同步为多条 Product

如果这些计划属于同一产品线，可以再利用 Creem 的 `Product Bundles` 能力，把这些 Creem Product 组织成一个 Bundle，用于升级 / 降级或统一展示。

## 5.5 Creem 映射结论

Creem 与 Kiro 当前模型不是一一对称映射，而是：

- Stripe：`内部商品 -> Stripe Product`，`内部计划 -> Stripe Price`
- Creem：`内部商品 + 内部计划 -> 一个 Creem Product`

所以如果同时接 Stripe 和 Creem，Kiro 当前模型依然合理，但同步层必须区分 provider：

- 对 Stripe 走“拆分同步”
- 对 Creem 走“合并同步”

## 6. 当前 schema 与 provider ID 的关系

当前 `products` / `product_plans` 表里还没有保存这些 provider 对象 ID，例如：

- `stripe_product_id`
- `stripe_price_id`
- `creem_product_id`

因此当前文档描述的是“语义映射关系”，不是“数据库里已经落好的 provider 主键关系”。

如果后续要正式接入 Stripe / Creem，建议不要直接把多个 provider 的外部 ID 都塞回核心商品表，而是单独设计 provider 映射表，例如：

- `product_provider_mappings`
- `product_plan_provider_mappings`

或者在一个统一表里记录：

- `provider`
- `product_id`
- `product_plan_id`
- `provider_product_id`
- `provider_price_id`
- `provider_status`

这样可以避免核心目录表被不同支付渠道字段污染。

## 7. 推荐同步策略

结合当前项目模型，推荐采用下面的同步策略：

### 7.1 Stripe

- 以 `products` 为主，同步 Stripe `Product`
- 以 `product_plans` 为主，同步 Stripe `Price`
- `product_code`、`plan_code` 作为稳定业务映射键

### 7.2 Creem

- 以 `products + product_plans` 组装 Creem `Product`
- 一个内部商品如果有多个计划，则在 Creem 中通常会变成多个 Product
- 如需表达同一产品线的月付 / 年付 / 高低档关系，可再使用 Creem `Product Bundles`

## 8. 外部参考

以下官方文档用于支撑本文档中的映射说明：

- Stripe:
  [How products and prices work](https://docs.stripe.com/products-prices/how-products-and-prices-work)
- Stripe:
  [The Price object](https://docs.stripe.com/api/prices/object)
- Stripe:
  [The Product object](https://docs.stripe.com/api/products/object)
- Creem:
  [Create Product](https://docs.creem.io/api-reference/endpoint/create-product)
- Creem:
  [Checkout API](https://docs.creem.io/features/checkout/checkout-api)
- Creem:
  [Product Bundles](https://docs.creem.io/features/product-bundles)
