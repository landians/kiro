
## 1. 技术栈清单

| 类别 | 技术 | 角色定位 | 规划说明 |
| --- | --- | --- | --- |
| 后端语言 | Rust 2024 Edition | 核心开发语言 | 项目统一语言标准 |
| HTTP 框架 | axum | 对外 API 接入层 | 负责路由、提取器、中间件、响应拼装 |
| 异步运行时 | tokio | 并发与 IO 基础 | 负责网络、定时任务、信号处理 |
| 错误处理 | thiserror + anyhow | 分层错误建模与聚合 | 领域错误显式建模，入口层统一聚合 |
| 可观测 | tracing + tracing-subscriber | 日志、追踪、诊断 | 统一上下文字段、请求链路与结构化日志 |
| 环境配置 | dotenv | 本地配置加载 | 支撑开发环境快速启动 |
| 内存分配器 | mimalloc | 运行时内存优化 | 作为全局分配器使用 |
| 主数据库 | PostgreSQL 17.0 | 核心事务数据存储 | 用户、订阅、支付、邀请等强一致业务数据 |
| 缓存 / 队列 | Redis 8.0 | 缓存、短态、消息流 | 登录状态、幂等键、Redis Stream 事件 |
| 数据访问 | sqlx | SQL 执行与类型映射 | 直接表达查询意图，保持可控性 |
| 消息队列 | Redis Stream | 领域事件投递 | 支撑异步通知、邮件、回调后处理 |

## 2. 组件使用约束

### 2.1 Rust 2024 Edition

- 全项目统一使用 Rust 2024 Edition。
- 原则上不允许以兼容旧习惯为由引入过时模式。
- 涉及 trait 异步接口时，优先采用原生能力与项目既定方案，不退回 `async_trait`。

### 2.2 axum

- `axum` 只用于 `interfaces/` 层和入口装配。
- Handler 负责参数解析、认证上下文读取、调用应用服务、组装响应。
- Handler 不直接拼接 SQL、不直接访问 Redis、不直接调用 Stripe/Creem SDK。

### 2.3 tokio

- 所有异步 IO、信号处理、后台任务执行统一依赖 `tokio`。
- 后台任务应具备可观测字段、取消能力和清晰生命周期。
- 阻塞型逻辑不得直接运行在核心异步执行路径上。

### 2.4 thiserror + anyhow

- `domain/` 中的业务错误、校验错误、状态错误使用 `thiserror` 显式定义。
- `application/` 可以聚合多个领域错误，但仍应尽量保留原始语义。
- `main.rs`、任务入口、初始化流程可使用 `anyhow::Result` 聚合外部错误。

### 2.5 tracing + tracing-subscriber

- 每个 HTTP 请求必须带有请求级 trace 上下文。
- 关键业务路径至少记录：用户标识、模块名、动作名、结果状态、耗时、错误码。
- 关键异步消费链路必须记录消息 ID、幂等键、重试次数、来源事件。
- 日志输出格式在开发环境可偏可读，在生产环境应优先结构化。

### 2.6 dotenv

- `dotenv` 仅作为本地开发便利层，不替代正式环境配置管理。
- 应在应用启动阶段统一加载配置，不在业务代码中按需散读环境变量。
- JWT 密钥、签名算法、TTL 和黑名单策略等也必须纳入统一配置模型。

### 2.7 mimalloc

- 保持为全局分配器。
- 若后续引入额外运行时组件，需要验证其与 `mimalloc` 的兼容性及收益。

### 2.8 PostgreSQL 17.0

- PostgreSQL 承担所有核心事务数据。
- 账号、身份绑定、邀请、订阅、支付记录等强一致数据优先入 PostgreSQL。
- 数据模型设计应避免把 Redis 当成事实来源。

### 2.9 Redis 8.0 + Redis Stream

- Redis 用于缓存、短生命周期状态、登录挑战辅助数据、限流辅助数据、幂等控制及消息流。
- 邮箱验证码和魔法链接令牌建议使用 Redis 临时键实现，并配合 TTL 与限流控制。
- Redis Stream 用于承接领域事件后的异步任务，如邮件发送、通知分发、账单后处理。
- Redis 中的数据必须具备 TTL 策略或消费清理策略，避免长期脏数据堆积。

### 2.10 sqlx

- 数据访问统一通过 `sqlx` 完成。
- 查询语句应按模块组织，避免“万能仓储文件”。
- 事务边界由 `application/` 控制，底层仓储实现只负责执行。

## 3. 基础设施分层建议

### 3.1 配置子系统

建议建设统一配置对象，至少包含：

- HTTP 服务监听配置。
- PostgreSQL 连接配置。
- Redis 连接配置。
- JWT / Refresh Token 鉴权配置。
- 第三方登录配置。
- Stripe / Creem 配置。
- 邮件服务配置。
- 观测与日志输出配置。

JWT 相关配置建议至少包含：

- 签发者和受众配置。
- JWT 签名算法与签名密钥。
- Access Token TTL，默认 7 天。
- Refresh Token TTL，默认 30 天。
- User-Agent 摘要绑定策略。
- 黑名单来源与热加载策略。

### 3.2 数据访问子系统

建议按模块拆分基础设施实现，例如：

- `infrastructure/persistence/postgres/accounts`
- `infrastructure/persistence/postgres/subscriptions`
- `infrastructure/persistence/redis/auth`
- `infrastructure/messaging/redis_stream`

这样可以避免基础设施层失控演变为“所有实现都堆在一起”的目录。

### 3.3 第三方集成子系统

建议将外部能力封装为明确适配器，而不是在应用层直接散落调用：

- `infrastructure/payments/stripe`
- `infrastructure/payments/creem`
- `infrastructure/auth/google`
- `infrastructure/auth/facebook`
- `infrastructure/email/...`

对于鉴权基础设施，建议增加：

- `infrastructure/auth/jwt`
- `infrastructure/auth/blacklist`

## 4. 非功能性基线

### 4.1 可观测基线

每个关键路径至少满足以下要求：

- 可定位请求来源。
- 可关联业务实体标识。
- 可识别错误类别与失败位置。
- 可关联重试、补偿、异步消费链路。

### 4.2 性能与稳定性基线

建议在项目早期即建立以下基线：

- 数据库连接池与 Redis 连接池配置化。
- 第三方回调和重复提交路径具备幂等控制。
- 异步任务具备死信或人工补偿策略。
- 关键写路径有超时、重试和熔断边界。
- JWT 密钥、签名算法、TTL 和黑名单策略必须配置化且可启动校验。
- User-Agent 绑定策略必须具备统一摘要算法和失败处理策略。
- 黑名单配置变更必须可自动加载到内存并具备观测日志。
