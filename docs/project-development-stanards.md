# Kiro 开发规范与架构约束

## 1. 文档范围

本文档用于细化 Kiro 项目的编码规范、分层目录规范、接口规范和可维护性要求。它是实现阶段的约束文档，而不是单纯的编码建议。

补充说明：

- 当前项目的 DDD 分层约束，额外参考 `food-app-server` 的目录组织和依赖方向：[victorsteven/food-app-server](https://github.com/victorsteven/food-app-server)。
- 该参考项目可以归纳出三个可直接落地的结构特征：
  - `application/ 只承载用例编排，通过领域仓储接口完成业务流程，不直接触碰 HTTP 和数据库驱动。
  - `domain/repository/` 定义仓储接口，`infrastructure/persistence` 提供具体实现，组合根统一完成装配。
  - `interfaces/` 与 `interfaces/middleware` 位于最外层，只负责输入输出、认证上下文和横切能力，不向下绕过 application 直接访问持久化实现。
- 参考重点不是照搬语言细节，而是将上述依赖方向翻译为当前 Rust 项目的可执行规范，并长期保持 `interfaces -> application -> domain/infrastructure` 的稳定边界。

## 2. Rust 编码规范

### 2.1 语言与安全

- 默认使用 Rust 2024 Edition。
- 实现层禁止 `unsafe`，除非已经通过单独 ADR 评审并给出使用边界、风险和替代方案分析。
- 公共 API 与核心业务代码优先强调可读性、可测试性与类型表达，不以“少写几行代码”为主要目标。

### 2.2 异步编程规范

- 运行时 API 直接使用原生 `async fn`。
- trait 异步接口统一使用 `fn -> impl Future`，不使用 async_trait，实现接口时，需要使用 `async fn` 来实现。
- 静态分发优先，运行时不依赖 `dyn` 后端对象，优先通过泛型完成编译期分发。
- 如确实需要引入动态分发，必须说明原因，例如插件式加载或运行时选择策略，并单独评审。

### 2.3 错误处理规范

- 领域层错误必须使用 `thiserror` 明确定义，禁止直接返回无语义字符串错误。
- `anyhow` 仅用于应用层、入口层或工具层的错误聚合，不作为领域错误的长期表达方式。
- 错误类型必须能够支持日志、HTTP 响应、告警与后续排障的最小信息需求。
- 跨层传递错误时，优先保留原始错误语义，不在中间层无差别丢失上下文。

### 2.4 可观测性规范

每条关键路径都必须包含三类可观测信息：

- 日志：记录关键动作、状态变化、失败原因。
- 指标：记录请求量、失败量、耗时、重试等关键统计信息。
- 追踪字段：记录请求级 trace 信息和核心业务标识。
- 监控数据采集方式：统一遵循 OpenTelemetry 规范。

最低要求如下：

- 每个 HTTP 请求都必须具备 trace 上下文。
- 每个关键业务动作都必须包含模块名、动作名、结果状态。
- 每个失败路径都必须能关联到错误类别与请求 trace。

## 3. 项目目录规范

### 3.1 `interfaces/`

职责：

- HTTP 路由定义。
- 请求解析与参数校验。
- 响应对象组装。
- 中间件。
- 认证上下文提取。

禁止事项：

- 写业务规则。
- 直接访问数据库。
- 直接调用第三方支付、邮件、登录 SDK。

判断标准：

如果一段逻辑脱离 HTTP 仍然成立，它就不应该放在 `interfaces/`。

#### 3.1.1 `interfaces/controller` 与 `interfaces/middleware` 的依赖边界清单

`interfaces/controller/` 允许依赖：

- `crate::application::*` 暴露出的 service 或 use case 入口。
- `crate::interfaces::dto::*`、`crate::interfaces::response::*` 这类接口层自有类型。
- `crate::interfaces::AppState`、请求提取器、响应封装、HTTP 状态码等框架对象。
- 与请求上下文直接相关的轻量对象，例如 `RequestTrace`、已解析的认证上下文。

`interfaces/controller/` 禁止依赖：

- `crate::infrastructure::persistence::*`、`sqlx::*`、`redis::*` 等底层技术实现。
- 任何数据库连接池、Redis client、外部 SDK client。
- 承担持久化语义的 repository 实现类型。
- 为了“少包一层”而在 controller 中重写业务流程、事务流程、权限判定主逻辑。

`interfaces/middleware/` 允许依赖：

- trace、鉴权、限流、审计、请求头处理等横切能力所需的轻量 service。
- `AppState` 中已经组装好的 application service。
- 与身份提取直接相关的 DTO、claims、上下文扩展类型。

`interfaces/middleware/` 禁止依赖：

- 直接创建或持有 `PgPool`、Redis 连接、第三方 SDK client。
- 编写领域规则或复杂业务编排，例如“创建管理员并发送通知”。
- 直接调用 repository 实现跳过 application service。

落地要求：

- controller 负责“把 HTTP 请求翻译为 application 输入，再把 application 输出翻译为 HTTP 响应”。
- middleware 负责“在请求进入 controller 前后处理横切关注点”，例如 `X-Trace-Id`、Access Token 校验、管理员身份注入。
- 如果某段逻辑既被 controller 调用又被 middleware 调用，优先上提到 `application/` 或专门的共享 service，而不是在接口层复制。

### 3.2 `application/`

职责：

- 编排用例。
- 定义事务边界。
- 组合多个领域对象与仓储接口。
- 协调同步调用和异步事件发布。
- 对外暴露按模块划分的 application service，例如 `application/auth.rs`、`application/health.rs`。
- `application/mod.rs` 只负责导出模块和组合 service 容器，不在 `mod.rs` 内直接堆放某个模块的完整 service 实现。

禁止事项：

- 依赖具体第三方 SDK。
- 把领域规则硬编码为流程脚本。
- 在 `application/mod.rs` 中直接定义大体量模块 service 实现。

判断标准：

`application/` 负责“怎么完成一次用例”，但不负责“业务规则本身是什么”。

进一步约束：

- 单个模块的 application service 必须落在独立文件或子目录中，例如 `application/auth.rs` 或 `application/auth/mod.rs`。
- `interfaces/` 只能依赖 `application` 暴露的 service，不能绕过 service 直接读取基础设施对象。
- 如果某项能力只是启动期组合容器，而不承载业务语义，应留在组合根或装配代码中，而不是伪装成 application service。

#### 3.2.1 application 层命名规范

结合参考项目中 `user_app.go` 这类薄应用层文件，当前 Rust 项目统一采用“按业务模块命名”的方式：

- 文件命名优先使用模块语义名，例如 `application/auth.rs`、`application/admin.rs`、`application/health.rs`，而不是 `application/auth_service_impl.rs` 这类技术实现名。
- 当某模块只有一个主要 application service 时，文件内主类型统一命名为 `XxxService`，例如 `AuthService`、`AdminService`。
- 当某模块变复杂时，升级为子目录：`application/admin/mod.rs`、`application/admin/commands.rs`、`application/admin/queries.rs`、`application/admin/service.rs`。
- `application/mod.rs` 只做模块导出、聚合容器定义和组合辅助，不直接堆放某个模块的长实现。
- 方法命名以用例动作命名，优先体现业务意图，例如 `create_admin`、`revoke_session_tokens`、`list_admin_users`，避免 `handle_request`、`process` 这类模糊命名。

#### 3.2.2 application 层文件模板

新增 application 模块时，默认遵循以下模板：

```rust
use crate::domain::repository::admin_repository::AdminRepository;
use crate::domain::service::password_hasher::PasswordHasher;

#[derive(Clone)]
pub struct AdminService<R, H> {
    admin_repository: R,
    password_hasher: H,
}

impl<R, H> AdminService<R, H>
where
    R: AdminRepository,
    H: PasswordHasher,
{
    pub fn new(admin_repository: R, password_hasher: H) -> Self {
        Self {
            admin_repository,
            password_hasher,
        }
    }

    pub async fn create_admin(&self, command: CreateAdminCommand) -> Result<AdminDto, AdminError> {
        // 1. 参数规整
        // 2. 调用领域规则/仓储接口
        // 3. 组织返回结果
        todo!()
    }
}
```

模板约束：

- 字段只保存该模块完成用例所必需的依赖，优先是 repository trait、领域服务、事件发布器、鉴权服务等。
- `new(...)` 只做依赖注入，不在构造函数中执行数据库连接、网络调用或迁移。
- application 方法负责用例编排、事务边界和跨依赖协调，不直接拼接 SQL，也不直接读取 HTTP 头。
- 输入输出优先使用 command/query/result 或 DTO，避免把 `axum::extract::*`、`HeaderMap` 等接口层对象带入 application。
- application 层可以组合多个 repository trait，但不应该依赖具体的 `PostgresAdminRepository` 之类实现类型。

### 3.3 `domain/`

职责：

- 实体。
- 值对象。
- 领域服务。
- 仓储 trait。
- 领域错误。

禁止事项：

- 依赖 `axum`。
- 依赖数据库驱动。
- 依赖第三方支付 SDK、社交登录 SDK、邮件 SDK。

判断标准：

如果替换 HTTP 框架、数据库或第三方服务后，这段规则仍然应该保留，那么它应该尽量位于 `domain/`。

### 3.4 `infrastructure/`

职责：

- 提供数据库实现。
- 提供 Redis 实现。
- 提供第三方支付适配。
- 提供第三方登录适配。
- 提供邮件、消息队列、通知通道实现。

要求：

- 面向应用层或领域层定义的抽象接口提供实现。
- 吸收外部系统差异，不把 SDK 细节泄露到上层。
- 基础设施组件在可能情况下优先采用 Builder 模式构建，特别是涉及多配置项、可选参数、连接池、客户端和第三方适配器初始化的场景。
- 基础设施目录不再额外引入 `AppInfrastructure`、`AuthInfrastructure` 这类“再包一层”的聚合类型；优先直接暴露清晰的底层构件，例如 `JwtService`、`TokenBlacklistService`、`PgPool`、`redis::Client`。
- 启动阶段可以返回资源集合或直接完成装配，但资源集合必须表达“已初始化资源”，不能替代 application service 本身。

Builder 模式建议适用于以下对象：

- 数据库连接池与仓储实现。
- Redis 客户端与消息消费者。
- JWT 签发器与验签器。
- 支付网关适配器、邮件发送适配器、第三方登录适配器。

采用该约束的原因：

- 避免构造函数参数过多导致可读性变差。
- 便于在启动阶段做配置校验和必填项约束。
- 便于测试环境按需覆盖部分配置。
- 有助于保持基础设施初始化逻辑的可扩展性和一致性。

进一步约束：

- `infrastructure/` 的职责是“提供实现”和“完成技术装配”，不是定义新的业务层抽象。
- 如果某个类型只是把多个基础设施对象机械地重新包成 `XxxInfrastructure`，通常应删除，改为直接传递真实依赖。
- repository、client、adapter、builder 的命名应直接表达其技术职责，而不是使用泛化的 `manager` 或 `infrastructure` 命名。

#### 3.4.1 `infrastructure/persistence` 的 repository 标准目录布局

参考 `food-app-server` 中“`domain/repository` 定义接口、`infrastructure/persistence/*_repository.go` 提供实现”的做法，当前项目统一采用如下持久化布局规则：

```text
src/
  domain/
    repository/
      admin.rs
      user.rs
  infrastructure/
    persistence/
      mod.rs
      postgres/
        mod.rs
        admin_repository.rs
        user_repository.rs
        migrations.rs
      redis/
        mod.rs
        token_blacklist_repository.rs
```

目录规则：

- `domain/repository/` 只定义抽象接口、查询参数和值对象，不包含 SQL、Redis key 规则和驱动类型。
- `infrastructure/persistence/postgres/` 放 PostgreSQL repository 实现、查询映射、事务辅助、migration 执行辅助。
- `infrastructure/persistence/redis/` 放 Redis 相关 repository、缓存存取、黑名单或分布式锁等实现。
- 当某后端只有 builder 和连接校验而尚未出现具体 repository 时，允许继续保留为单文件；一旦出现两个及以上 repository，实现目录必须展开为子目录而不是继续堆在 `postgres.rs` 中。

命名规则：

- trait 命名使用业务语义，例如 `AdminRepository`、`UserRepository`。
- PostgreSQL 实现命名为 `PostgresAdminRepository`、`PostgresUserRepository`。
- Redis 实现命名为 `RedisTokenBlacklistRepository`、`RedisIdempotencyRepository`。
- 文件名统一使用 snake_case，并与主类型保持一致，例如 `admin_repository.rs`、`token_blacklist_repository.rs`。

实现要求：

- repository 实现类型只能出现在 `infrastructure/persistence/**`，不得泄漏到 `interfaces/`。
- repository 构造函数只接收底层技术依赖与必要配置，例如 `PgPool`、`redis::Client`、表名前缀、key 前缀。
- repository 负责把数据库模型转换为领域对象或 persistence record，不把 `sqlx::Row`、驱动错误原样向上传播到 controller。
- 多 repository 共享连接池时，在组合根中复用同一个池，不在每个 repository 内自行建立新连接。

建议模板：

```rust
use sqlx::PgPool;

use crate::domain::repository::admin::AdminRepository;

#[derive(Clone)]
pub struct PostgresAdminRepository {
    pool: PgPool,
}

impl PostgresAdminRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AdminRepository for PostgresAdminRepository {
    // implement repository trait here
}
```

### 3.5 `main.rs`

职责：

- 应用入口。
- 配置加载。
- 日志与 tracing 初始化。
- 依赖装配。
- 运行时启动与优雅关闭。

禁止事项：

- 编写具体业务逻辑。
- 直接承载模块实现细节。

组合根约束：

- `main.rs` 与启动引导代码是组合根，负责把基础设施对象装配成 application services，再交给 `interfaces/`。
- 组合根可以调用 `infrastructure` builder、repository、adapter，也可以创建 `application` service，但不能把具体业务规则回写到入口层。
- `AppState` 只持有配置、application services 和纯运行时元数据，不直接持有底层基础设施对象。
- 参考项目中的 `main.go`，装配顺序固定为：配置加载 -> 基础设施初始化 -> repository / adapter 构建 -> application service 构建 -> router/state 注入 -> 服务器启动。

## 4. 接口规范

### 4.1 RESTful API

- 对外接口遵循 RESTful API 风格。
- URL 应围绕资源命名，而不是围绕动作命名。
- HTTP 方法与语义保持一致，例如：
  - `GET` 用于查询。
  - `POST` 用于创建。
  - `PUT` / `PATCH` 用于更新。
  - `DELETE` 用于删除或撤销。
- 按照功能模块进行路由分组

### 4.2 `X-Trace-Id`

- 所有响应头必须包含 `X-Trace-Id`。
- 如果请求头中已经带有可接受的 trace 标识，可在校验后透传或映射。
- 如果请求头中没有 trace 标识，系统必须在入口处生成，并贯穿日志与响应头。
- 错误响应也不能缺失 `X-Trace-Id`。

### 4.3 响应与错误返回

基于当前规划，建议统一约束如下：

- 成功响应结构保持稳定，避免同类接口返回风格不一致。
- 错误响应至少包含：错误码、错误消息、`X-Trace-Id`。
- 面向客户端的错误消息与内部诊断日志应适度分离。
- 统一接口约定参考：[Kiro API 接口约定](project-api-contract-standards.md)。

### 4.4 鉴权规范

- 受保护接口统一使用 JWT Bearer Token 鉴权。
- 鉴权采用无状态 JWT + Refresh Token 模型，不使用数据表存储 JWT 或 Refresh Token。
- JWT 默认有效期为 2 小时。
- Refresh Token 默认有效期为 15 天。
- Refresh Token 用于换取新的 JWT，不直接用于访问业务接口。
- 客户端持有 JWT 和 Refresh Token；业务接口通过 `Authorization: Bearer <jwt>` 传递 JWT，刷新接口通过专用请求头或等价安全方式传递 Refresh Token。
- JWT 中必须包含 `sub`、`jti` 和 User-Agent 摘要字段，例如 `ua_hash`。
- Refresh Token 中也必须包含 `sub`、`jti` 和 User-Agent 摘要字段，例如 `ua_hash`。
- 接口层鉴权中间件必须校验签名、过期时间和 `ua_hash` 一致性。
- 所有资源操作相关接口必须通过黑名单校验中间件，黑名单命中后直接拒绝访问。
- 当用户登出、令牌被吊销或出现高风险安全事件时，通过配置化黑名单机制阻断对应 JWT 或 Refresh Token。

说明：

- 将 User-Agent 摘要写入 JWT 只能作为令牌绑定和异常检测辅助手段，不能替代限流、风控和审计。

## 5. 补充规范

### 5.1 测试规范

- 领域层优先编写纯业务单元测试。
- 应用层补充用例编排测试与事务边界测试。
- 接口层补充路由和响应契约测试。
- 基础设施层补充与 PostgreSQL、Redis、第三方适配相关的集成测试。

### 5.2 幂等性规范

- 支付回调、邀请接受、邮件补发、消息消费等路径必须具备幂等保障。
- 幂等键可以放在 PostgreSQL 或 Redis，但必须有明确归属与过期策略。

### 5.3 安全治理规范

- 鉴权相关密钥、JWT 签名配置和黑名单策略必须配置化管理。
- 多环境或多服务共享 Redis 时，必须通过统一的 `REDIS_KEY_PREFIX` 做 key 空间隔离，避免黑名单等安全数据互相污染。
- User-Agent 建议以标准化摘要形式存储，不建议直接在 JWT 中保留冗长原始字符串。
- 对刷新失败、User-Agent 不匹配、频繁刷新的行为应具备审计日志与限流策略。
- 黑名单配置变更后，程序必须自动重新加载到内存，并对后续请求立即生效。

### 5.4 数据库设计规范

- PostgreSQL 业务表主键统一使用数据库自增主键，推荐 `bigint generated always as identity`。
- 外键字段类型必须与目标主键保持一致，统一使用 `bigint`。
- 不在核心业务表中混用 `uuid` 主键和自增主键，避免 schema 风格不一致。
- 自增主键只承担内部关系标识职责；如需要对外稳定标识，可额外设计 `*_code` 字段。

### 5.5 文档治理规范

- 新增模块前先更新计划文档。
- 修改公共接口前先更新接口契约说明。
- 引入例外设计时优先新增 ADR，而不是直接在代码里“先做再说”。

### 5.6 代码扁平化编写规范
参考：[代码扁平化规范](code-flattening-guideline.md)

### 5.7 配置定义规范
- 定义 ENV 配置时，不需要使用项目名称作为配置名前缀

## 6. 实施检查清单

每个模块开发前建议自查以下问题：

- 这段逻辑是否放在正确分层？
- 是否引入了不该出现的框架或 SDK 依赖？
- 是否具备错误类型与追踪字段？
- 是否能在响应头返回 `X-Trace-Id`？
- 是否正确接入 JWT 鉴权和刷新令牌校验？
- 是否覆盖了关键测试路径？

## 7. 维护规则

- 修改分层职责时，必须同步更新本文件。
- 若代码实现与本文件产生偏差，应优先判断是代码违规还是规范需要升级，避免长期失配。
- 若新增例外规则，必须在本文件标注范围和原因，或以 ADR 形式独立记录。
