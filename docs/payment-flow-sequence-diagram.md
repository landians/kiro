# Kiro 支付流程时序图

## 1. 文档目的

本文档用于描述 Kiro 项目中“用户购买商品并完成支付”的目标流程时序。

- 本文档是面向当前项目规划的目标设计稿，不代表仓库中已经全部实现。
- 当前优先描述 Stripe 作为首个支付渠道的完整链路。
- 如果后续接入 Creem，整体编排可保持一致，只替换支付提供方适配器与对应 webhook 事件。

## 2. 适用范围

本文档覆盖以下阶段：

- 商品展示与用户发起购买
- 下单前购买资格校验
- 创建支付订单与价格快照
- 跳转 Stripe Checkout 完成支付
- Stripe Webhook 回写支付结果
- 支付成功后开通订阅或发放一次性权益
- 前端查询订单结果
- 退款 / 争议等逆向链路

## 3. 参与者

- `User`：终端用户
- `Frontend`：前端站点或客户端
- `Kiro API`：统一业务接口层
- `Purchase Validation`：购买前置校验层
- `Order Service`：订单与支付状态编排层
- `Postgres`：订单、商品、订阅、账单等持久化存储
- `Stripe Adapter`：Stripe SDK / API 适配层
- `Stripe`：Stripe 平台
- `Webhook Handler`：Stripe 回调验签、去重与事件路由层
- `Entitlement / Subscription Service`：权益开通与订阅状态维护层

## 4. 主时序图

```mermaid
sequenceDiagram
    autonumber
    actor User as User
    participant Frontend as Frontend
    participant API as Kiro API
    participant Validation as Purchase Validation
    participant Order as Order Service
    participant DB as Postgres
    participant StripeAdapter as Stripe Adapter
    participant Stripe as Stripe
    participant Webhook as Webhook Handler
    participant Entitlement as Entitlement / Subscription Service

    rect rgb(245, 248, 252)
        Note over User,API: 阶段 A：商品展示与发起购买
        User->>Frontend: 浏览商品与套餐
        Frontend->>API: GET /products
        API->>DB: 查询 active products + product_plans
        DB-->>API: 返回商品与套餐
        API-->>Frontend: 返回商品展示数据
        User->>Frontend: 选择 plan_code 并点击购买
    end

    rect rgb(250, 247, 240)
        Note over Frontend,DB: 阶段 B：创建订单前校验与价格快照
        Frontend->>API: POST /orders { plan_code }
        API->>Validation: validate(user_id, plan_code)
        Validation->>DB: 查询 active plan
        DB-->>Validation: 返回 plan
        Validation->>DB: 查询 active product
        DB-->>Validation: 返回 product
        Validation-->>API: 返回 ValidatedPurchase

        API->>Order: create_pending_order(validated_purchase)
        Order->>DB: 写入 payment_order\n状态 = pending\n写入商品/计划/价格快照
        DB-->>Order: 返回 order_id / order_no
        Order-->>API: 返回 pending order
    end

    rect rgb(240, 249, 244)
        Note over API,Stripe: 阶段 C：创建 Stripe Checkout Session
        API->>StripeAdapter: create_checkout_session(order_snapshot)
        StripeAdapter->>Stripe: 创建 customer / product reference / price reference / checkout session
        Stripe-->>StripeAdapter: 返回 checkout_session_id + checkout_url
        StripeAdapter-->>API: 返回 session 信息
        API->>Order: bind_provider_session(order_id, session_id)
        Order->>DB: 更新 payment_order.provider_session_id
        DB-->>Order: 更新成功
        API-->>Frontend: 返回 order_no + checkout_url
        Frontend-->>User: 跳转 Stripe Checkout
    end

    rect rgb(252, 246, 246)
        Note over User,Stripe: 阶段 D：用户在 Stripe 完成支付或取消支付
        User->>Stripe: 填写支付信息并确认
        alt 支付成功
            Stripe-->>User: 展示支付成功页
        else 用户取消或支付失败
            Stripe-->>User: 展示取消页或失败页
        end
    end

    rect rgb(243, 244, 252)
        Note over Stripe,DB: 阶段 E：Webhook 异步回写结果
        Stripe->>Webhook: POST /payments/webhooks/stripe
        Webhook->>StripeAdapter: verify_signature(payload, signature)
        StripeAdapter-->>Webhook: 验签成功

        Webhook->>DB: 查 payment_webhook_events(provider, event_id)
        alt event 已处理
            DB-->>Webhook: 已存在
            Webhook-->>Stripe: 200 OK
        else 首次处理
            DB-->>Webhook: 不存在
            Webhook->>DB: 写入 payment_webhook_events(event_id, type, payload, status)

            alt one_time 支付成功事件
                Webhook->>Order: mark_paid(order_no, provider_payment_id)
                Order->>DB: payment_order: pending -> paid
                DB-->>Order: 更新成功
                Webhook->>Entitlement: grant_one_time_entitlement(order_snapshot)
                Entitlement->>DB: 写入权益 / 发票 / 交付记录
                DB-->>Entitlement: 完成
            else subscription 首次支付成功事件
                Webhook->>Order: mark_paid(order_no, provider_payment_id)
                Order->>DB: payment_order: pending -> paid
                DB-->>Order: 更新成功
                Webhook->>Entitlement: activate_subscription(order_snapshot, period)
                Entitlement->>DB: 写入 subscription / invoice / entitlement
                DB-->>Entitlement: 完成
            else checkout 过期 / 支付失败 / 用户取消
                Webhook->>Order: mark_failed_or_canceled(order_no, reason)
                Order->>DB: payment_order: pending -> failed/canceled
                DB-->>Order: 更新成功
            end

            Webhook-->>Stripe: 200 OK
        end
    end

    rect rgb(247, 250, 247)
        Note over User,DB: 阶段 F：前端确认支付结果
        User->>Frontend: 返回站点成功页 / 结果页
        Frontend->>API: GET /orders/{order_no}
        API->>DB: 查询 payment_order + subscription + entitlement
        DB-->>API: 返回聚合结果
        API-->>Frontend: 返回最终支付状态与权益状态
        Frontend-->>User: 展示“支付成功 / 处理中 / 已取消 / 支付失败”
    end

    rect rgb(252, 248, 242)
        Note over Stripe,DB: 阶段 G：退款 / 争议等逆向链路
        opt 发生退款、拒付、订阅取消或账单逆向事件
            Stripe->>Webhook: POST /payments/webhooks/stripe
            Webhook->>StripeAdapter: verify_signature(...)
            StripeAdapter-->>Webhook: 验签成功
            Webhook->>DB: 去重检查 event_id
            DB-->>Webhook: 可处理
            Webhook->>Entitlement: revoke_or_adjust_entitlement(...)
            Entitlement->>DB: 更新 refund / invoice / subscription / entitlement 状态
            DB-->>Entitlement: 完成
            Webhook-->>Stripe: 200 OK
        end
    end
```

## 5. 关键状态说明

### 5.1 订单状态建议

- `pending`：订单已创建，尚未确认支付结果。
- `paid`：已确认支付成功。
- `failed`：支付失败。
- `canceled`：用户取消、会话过期，或被业务主动关闭。
- `refunded`：已退款，通常由逆向流程驱动。

### 5.2 设计重点

- 下单时必须写入商品、计划、金额、币种、收费方式等快照，避免后续商品改价影响历史订单。
- 前端跳转成功页不能直接视为支付成功，最终状态必须以 webhook 异步回写为准。
- webhook 处理必须具备验签、去重、幂等更新和可重复消费能力。
- 订阅型商品与一次性商品应共享“下单与支付确认”主链路，但在支付成功后的履约动作不同。
- 退款、争议、取消订阅、续费失败等逆向事件也必须通过统一 webhook 编排进入状态机。

## 6. 建议落地接口

如果按当前项目路线推进，建议优先落地以下接口：

- `GET /products`
- `GET /products/{product_code}`
- `POST /orders`
- `GET /orders/{order_no}`
- `POST /payments/webhooks/stripe`

## 7. 推荐实施顺序

结合当前仓库进度，建议按以下顺序实现：

1. 补齐支付订单表与价格快照模型。
2. 在 `kiro-api` 中实现 `POST /orders`，接入购买前置校验层。
3. 实现 Stripe Adapter 与 Checkout Session 创建。
4. 实现 Stripe Webhook 验签、去重、事件分发。
5. 实现支付成功后的订阅开通 / 一次性权益发放。
6. 实现 `GET /orders/{order_no}` 供前端结果页轮询与确认。
