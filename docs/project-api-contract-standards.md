# Kiro API 接口约定

## 1. 文档范围

本文档用于固定 Kiro 当前已经落地的 HTTP 接口响应约定，重点约束以下内容：

- 成功响应结构
- 错误响应结构
- HTTP 状态码使用原则
- `X-Trace-Id` 请求头与响应头约定
- 当前已定义错误码
- 后续新增接口时必须保持一致的实现边界

本文档的目标不是描述具体业务接口，而是约束所有接口共享的基础契约，避免后续模块开发时出现返回结构漂移。

## 2. 适用范围

本约定适用于当前项目内所有对外 HTTP 接口，包括但不限于：

- 公共接口
- 登录与鉴权接口
- 商品、支付、订阅接口
- 管理后台接口
- 健康检查接口

如果某个接口需要例外，必须先更新本文档或补充单独 ADR，不能直接在实现中私自偏离。

## 3. 总体原则

- 所有接口都必须返回 JSON 响应。
- 所有响应都必须带有 `X-Trace-Id` 响应头。
- 成功响应和错误响应必须使用统一 envelope，不能各模块自定义外层结构。
- HTTP 状态码表达协议语义，响应体表达业务和诊断语义，两者必须同时保持一致。
- 面向客户端的错误消息应简洁稳定；内部排障细节不直接暴露给客户端。

## 4. 成功响应约定

### 4.1 外层结构

成功响应统一使用以下结构：

```json
{
  "success": true,
  "data": {}
}
```

约束如下：

- `success` 固定为 `true`
- `data` 承载具体业务响应
- `data` 的内部结构由具体接口定义，但外层 envelope 不允许变化

### 4.2 示例

`GET /health/live`

```json
{
  "success": true,
  "data": {
    "status": "ok",
    "service": "kiro",
    "runtime_env": "local",
    "uptime_seconds": 5
  }
}
```

### 4.3 关于非 2xx 成功 envelope

当前项目存在一种特殊但明确允许的情况：

- `/health/ready` 在依赖未就绪时返回 `503 Service Unavailable`
- 但响应体仍然使用成功 envelope，即 `success: true`

原因：

- readiness 接口本质是在返回系统状态快照
- 即使系统当前“不就绪”，请求本身仍被成功处理，并正确返回了状态数据

因此，`success` 字段表示“响应 envelope 是否成功构建”，不是简单等价于“HTTP 状态码是否为 2xx”。

后续除状态型接口外，普通业务接口不应随意复用这种模式。

## 5. 错误响应约定

### 5.1 外层结构

错误响应统一使用以下结构：

```json
{
  "success": false,
  "error": {
    "code": "error_code",
    "message": "Human readable message.",
    "trace_id": "trace_id_value"
  }
}
```

约束如下：

- `success` 固定为 `false`
- `error.code` 是稳定错误码，供客户端识别
- `error.message` 是面向客户端的可读消息
- `error.trace_id` 必须与响应头 `X-Trace-Id` 保持一致

### 5.2 示例

未命中路由时：

```json
{
  "success": false,
  "error": {
    "code": "route_not_found",
    "message": "The requested route does not exist.",
    "trace_id": "cli_trace_123"
  }
}
```

## 6. HTTP 状态码约定

### 6.1 基本规则

- `200 OK`：普通读取、普通成功响应
- `201 Created`：资源已创建
- `204 No Content`：无响应体的删除或幂等确认
- `400 Bad Request`：请求格式错误、参数非法、客户端输入不满足要求
- `401 Unauthorized`：未认证或认证令牌无效
- `403 Forbidden`：已认证但没有权限
- `404 Not Found`：资源不存在，或路由不存在
- `409 Conflict`：状态冲突、重复提交、幂等冲突
- `422 Unprocessable Entity`：请求格式合法，但业务校验失败
- `429 Too Many Requests`：触发限流
- `502 Bad Gateway`：上游第三方依赖调用失败
- `500 Internal Server Error`：未分类内部错误
- `503 Service Unavailable`：依赖未就绪、服务临时不可用

### 6.2 当前已实现状态码

当前代码中已经落地的状态码包括：

- `200 OK`
- `400 Bad Request`
- `401 Unauthorized`
- `404 Not Found`
- `409 Conflict`
- `502 Bad Gateway`
- `500 Internal Server Error`
- `503 Service Unavailable`

后续新增状态码时，必须同步更新本文档。

## 7. `X-Trace-Id` 约定

### 7.1 请求头规则

客户端可以主动传入：

```http
X-Trace-Id: cli_trace_123
```

系统当前接受的 trace id 规则为：

- 非空
- 长度不超过 64
- 只允许字母、数字、`-`、`_`

如果客户端传入值不合法，系统会忽略该值并自动生成新的 trace id。

### 7.2 响应头规则

所有响应都必须返回：

```http
X-Trace-Id: <trace_id>
```

包括：

- 成功响应
- 业务错误响应
- 路由未命中错误响应
- 健康检查响应

### 7.3 透传规则

- 如果请求携带合法 `X-Trace-Id`，响应头和错误体中的 `trace_id` 必须使用同一个值
- 如果请求未携带合法 `X-Trace-Id`，系统必须自动生成，并在响应头中返回

### 7.4 与日志的关系

请求完成日志应使用同一个 trace id 输出，保证以下三者可以关联：

- 请求日志
- 响应头 `X-Trace-Id`
- 错误响应体中的 `error.trace_id`

## 8. 鉴权请求头约定

### 8.1 Access Token

受保护业务接口统一使用：

```http
Authorization: Bearer <access_token>
```

当前受保护接口会同时要求：

- 合法 Access Token
- 合法 `User-Agent`
- `User-Agent` 与 token 中的 `ua_hash` 一致
- token 未命中黑名单

当前已落地的 Access Token 保护接口包括：

- `GET /auth/protected`
- `GET /auth/me`

### 8.2 Refresh Token

刷新接口当前使用专用请求头：

```http
X-Refresh-Token: <refresh_token>
```

同时要求请求携带：

```http
User-Agent: <user-agent>
```

当前 refresh 语义如下：

- 刷新接口只接受 Refresh Token，不接受 Access Token
- 刷新成功后返回新的 Access Token 和新的 Refresh Token
- 原 Refresh Token 会立即进入黑名单，避免重复使用
- 黑名单默认持久化在 Redis 中，并随 token 自然过期时间自动过期

### 8.3 Logout Token

登出接口当前要求同时显式提交 Access Token 和 Refresh Token：

```http
Authorization: Bearer <access_token>
X-Refresh-Token: <refresh_token>
User-Agent: <user-agent>
```

当前 logout 语义如下：

- 登出接口要求 Access Token 与 Refresh Token 都已通过各自鉴权校验
- 两个 token 必须属于同一个 `subject`
- 登出成功后，当前 Access Token 和当前 Refresh Token 都会立即进入黑名单
- 黑名单默认持久化在 Redis 中，并随 token 自然过期时间自动过期
- 显式登出后，再次使用这两个 token 访问受保护接口或刷新接口，都会返回 `token_revoked`

### 8.4 Google OAuth Callback

Google 登录回调接口当前为：

```http
GET /auth/google/callback?code=<authorization_code>&state=<oauth_state>
User-Agent: <user-agent>
```

当前回调校验规则如下：

- `code` 必须存在且非空
- `state` 必须存在且非空
- `state` 必须是服务端签发且未过期的 Google OAuth state
- `User-Agent` 必须存在，后续签发的会话 token 会继续绑定 `ua_hash`

当前 `state` 由服务端在 `GET /auth/google/authorization-url` 中签发，默认有效期由 `GOOGLE_OAUTH_STATE_TTL_SECONDS` 控制。

## 9. 当前错误码清单

### 9.1 已定义错误码

#### `route_not_found`

- HTTP 状态码：`404 Not Found`
- 含义：访问了不存在的路由
- 当前默认消息：`The requested route does not exist.`

#### `missing_bearer_token`

- HTTP 状态码：`401 Unauthorized`
- 含义：受保护接口缺少 Bearer Token
- 当前默认消息：`Authorization header must use Bearer token.`

#### `missing_user_agent`

- HTTP 状态码：
  - 鉴权中间件路径：`401 Unauthorized`
  - Google callback 参数校验路径：`400 Bad Request`
- 含义：请求缺少 `User-Agent`，无法完成 callback 绑定或 `ua_hash` 校验
- 当前默认消息：
  - 鉴权中间件路径：`User-Agent header is required for authenticated requests.`
  - Google callback 路径：`User-Agent header is required.`

#### `missing_refresh_token`

- HTTP 状态码：`401 Unauthorized`
- 含义：刷新接口缺少 `X-Refresh-Token`
- 当前默认消息：`Refresh token header is required.`

#### `invalid_access_token`

- HTTP 状态码：`401 Unauthorized`
- 含义：访问令牌无效，例如签名错误、格式错误或无法解析
- 当前默认消息：`Access token is invalid.`

#### `invalid_refresh_token`

- HTTP 状态码：`401 Unauthorized`
- 含义：刷新令牌无效，例如签名错误、格式错误或无法解析
- 当前默认消息：`Refresh token is invalid.`

#### `invalid_token_kind`

- HTTP 状态码：`401 Unauthorized`
- 含义：把 Refresh Token 当成 Access Token 使用，或令牌类型不匹配
- 当前默认消息：
  - Access Token 路径：`Access token is invalid for this endpoint.`
  - Refresh Token 路径：`Refresh token is invalid for this endpoint.`

#### `token_expired`

- HTTP 状态码：`401 Unauthorized`
- 含义：访问令牌已过期
- 当前默认消息：
  - Access Token 路径：`Access token has expired.`
  - Refresh Token 路径：`Refresh token has expired.`

#### `user_agent_mismatch`

- HTTP 状态码：`401 Unauthorized`
- 含义：当前请求的 `User-Agent` 与 token 中的 `ua_hash` 不匹配
- 当前默认消息：
  - Access Token 路径：`Access token does not match the current user agent.`
  - Refresh Token 路径：`Refresh token does not match the current user agent.`

#### `token_revoked`

- HTTP 状态码：`401 Unauthorized`
- 含义：访问令牌已命中黑名单
- 当前默认消息：
  - Access Token 路径：`Access token has been revoked.`
  - Refresh Token 路径：`Refresh token has been revoked.`

#### `token_refresh_failed`

- HTTP 状态码：`500 Internal Server Error`
- 含义：Refresh Token 已验证通过，但服务端刷新新 token 对失败
- 当前默认消息：`Failed to refresh session tokens.`

#### `authenticated_user_not_found`

- HTTP 状态码：`401 Unauthorized`
- 含义：Access Token 已通过校验，但 token subject 对应的当前用户不存在
- 当前默认消息：`Authenticated user does not exist.`

#### `current_user_lookup_failed`

- HTTP 状态码：`500 Internal Server Error`
- 含义：当前用户信息查询过程中发生服务端内部错误
- 当前默认消息：`Failed to load current user.`

#### `current_user_unavailable`

- HTTP 状态码：`503 Service Unavailable`
- 含义：当前用户 application service 未装配完成，服务暂时无法提供 `/auth/me`
- 当前默认消息：`Current user service is not available.`

#### `missing_authorization_code`

- HTTP 状态码：`400 Bad Request`
- 含义：Google 登录回调缺少授权码 `code`
- 当前默认消息：`Google authorization code is required.`

#### `missing_google_state`

- HTTP 状态码：`400 Bad Request`
- 含义：Google 登录回调缺少 `state`
- 当前默认消息：`Google oauth state is required.`

#### `invalid_google_state`

- HTTP 状态码：`400 Bad Request`
- 含义：Google 登录回调的 `state` 非法、已过期或不是服务端签发
- 当前默认消息：`Google oauth state is invalid or expired.`

#### `google_authorization_denied`

- HTTP 状态码：`400 Bad Request`
- 含义：Google 授权页主动拒绝授权并带回错误信息
- 当前默认消息：动态拼接 Google 返回的错误描述

#### `identity_binding_conflict`

- HTTP 状态码：`409 Conflict`
- 含义：当前 Google 身份尝试绑定到一个已被其他身份关系占用的邮箱/绑定关系
- 当前默认消息：`Google identity conflicts with an existing binding.`

#### `google_oauth_exchange_failed`

- HTTP 状态码：`502 Bad Gateway`
- 含义：服务端向 Google 交换授权码或拉取用户资料失败
- 当前默认消息：`Failed to exchange Google authorization code or fetch profile.`

#### `google_authorization_url_build_failed`

- HTTP 状态码：`500 Internal Server Error`
- 含义：Google 授权地址构建过程中发生服务端内部错误
- 当前默认消息：`Failed to build Google authorization url.`

#### `google_login_failed`

- HTTP 状态码：`500 Internal Server Error`
- 含义：Google 登录编排过程中出现未分类的内部错误，例如 profile 解析、仓储异常或 token 签发异常
- 当前默认消息：`Failed to complete Google login.`

#### `google_login_unavailable`

- HTTP 状态码：`503 Service Unavailable`
- 含义：Google 登录未启用，或 Google 登录基础设施未完成装配
- 当前默认消息：`Google login is not enabled.`

#### `token_subject_mismatch`

- HTTP 状态码：`401 Unauthorized`
- 含义：登出接口提交的 Access Token 与 Refresh Token 不属于同一个主体
- 当前默认消息：`Access token and refresh token must belong to the same subject.`

#### `blacklist_unavailable`

- HTTP 状态码：`503 Service Unavailable`
- 含义：Redis 黑名单后端不可用，服务无法安全完成 token 吊销或吊销校验
- 当前默认消息：`Token blacklist backend is unavailable.`

### 9.2 后续错误码命名规则

新增错误码时统一遵循以下规则：

- 使用小写字母和下划线
- 错误码表达“错误语义”，而不是表达 HTTP 状态码
- 错误码保持稳定，不能因为文案调整而改变

推荐命名形式：

- `invalid_request`
- `validation_failed`
- `unauthorized`
- `forbidden`
- `resource_not_found`
- `conflict`
- `rate_limited`
- `internal_error`

不推荐形式：

- `400`
- `bad_request_error_code_v2`
- `user_name_or_email_invalid_because_too_long`

## 10. 健康检查接口特殊约定

### 10.1 `GET /health/live`

语义：

- 表示进程存活
- 只要 HTTP 服务本身正常响应，就返回 `200`

响应结构：

```json
{
  "success": true,
  "data": {
    "status": "ok",
    "service": "kiro",
    "runtime_env": "local",
    "uptime_seconds": 5
  }
}
```

### 10.2 `GET /health/ready`

语义：

- 表示服务是否对外可接收业务流量
- 当前检查项包括：
  - `http_server`
  - `postgres`
  - `redis`

返回规则：

- 全部依赖就绪时返回 `200`
- 任一关键依赖未就绪时返回 `503`

响应结构示例：

```json
{
  "success": true,
  "data": {
    "status": "not_ready",
    "service": "kiro",
    "runtime_env": "local",
    "checks": {
      "http_server": { "status": "ok" },
      "postgres": {
        "status": "error",
        "message": "timed out while creating postgres connection pool"
      },
      "redis": {
        "status": "error",
        "message": "failed to open redis connection"
      }
    },
    "uptime_seconds": 2
  }
}
```

## 11. Google 授权地址接口约定

### 11.1 `GET /auth/google/authorization-url`

语义：

- 为前端生成 Google 登录跳转地址
- 同时返回服务端签发的 `state` 与对应 `nonce`
- `state` 用于 callback 防伪与过期校验，默认有效期由 `GOOGLE_OAUTH_STATE_TTL_SECONDS` 控制

请求头：

- 无强制鉴权头要求

成功响应结构：

```json
{
  "success": true,
  "data": {
    "authorization_url": "https://accounts.google.com/o/oauth2/v2/auth?...",
    "state": "signed_google_oauth_state",
    "nonce": "nonce_uuid"
  }
}
```

失败规则：

- Google 登录未启用或相关基础设施未装配时返回 `google_login_unavailable`
- 授权地址构建过程中发生内部错误时返回 `google_authorization_url_build_failed`

## 12. Google 登录回调接口约定

### 12.1 `GET /auth/google/callback`

语义：

- 接收 Google 授权完成后的回调参数
- 校验 `state`
- 调用 Google 交换授权码并拉取用户资料
- 完成首次建号、已有用户绑定或已有 identity 复用
- 成功后直接返回当前会话的 Access Token / Refresh Token

请求参数：

```http
GET /auth/google/callback?code=<authorization_code>&state=<oauth_state>
```

请求头：

```http
User-Agent: <user-agent>
```

成功响应结构：

```json
{
  "success": true,
  "data": {
    "user_code": "user_xxx",
    "identity_code": "identity_xxx",
    "provider": "google",
    "is_new_user": true,
    "access_token": "access_token",
    "refresh_token": "refresh_token",
    "access_token_expires_at": 1700000000,
    "refresh_token_expires_at": 1701296000
  }
}
```

失败规则：

- Google 授权页主动拒绝时返回 `google_authorization_denied`
- 缺少 `code` 时返回 `missing_authorization_code`
- 缺少 `state` 时返回 `missing_google_state`
- `state` 非法或过期时返回 `invalid_google_state`
- 缺少 `User-Agent` 时返回 `missing_user_agent`
- 已存在冲突绑定关系时返回 `identity_binding_conflict`
- 调用 Google 交换授权码或拉取用户资料失败时返回 `google_oauth_exchange_failed`
- Google 登录未启用或服务未装配时返回 `google_login_unavailable`
- 其余内部失败统一返回 `google_login_failed`

## 13. 当前用户接口约定

### 13.1 `GET /auth/me`

语义：

- 根据当前 Access Token 返回当前登录用户信息
- 作为 M1.3 最小“个人页 / 当前用户”查询接口

请求头：

```http
Authorization: Bearer <access_token>
User-Agent: <user-agent>
```

成功响应结构：

```json
{
  "success": true,
  "data": {
    "user_code": "user_42",
    "email": "hello@example.com",
    "display_name": "Hello User",
    "avatar_url": "https://example.com/avatar.png",
    "locale": "en-US",
    "time_zone": "Asia/Shanghai",
    "status": "active",
    "last_login_at": 1700000000,
    "created_at": 1700000000,
    "updated_at": 1700000000
  }
}
```

失败规则：

- 缺少或非法 Access Token 时返回 `missing_bearer_token` / `invalid_access_token`
- token 类型不匹配时返回 `invalid_token_kind`
- `User-Agent` 不合法或不匹配时返回 `missing_user_agent` / `user_agent_mismatch`
- token 已吊销时返回 `token_revoked`
- token 对应用户不存在时返回 `authenticated_user_not_found`
- 当前用户查询服务未装配时返回 `current_user_unavailable`
- 查询过程中发生内部错误时返回 `current_user_lookup_failed`

## 14. 刷新接口约定

### 14.1 `POST /auth/refresh`

语义：

- 使用 Refresh Token 换取新的 Access Token 和新的 Refresh Token
- 用于闭合 token 生命周期

请求头：

```http
X-Refresh-Token: <refresh_token>
User-Agent: <user-agent>
```

成功响应结构：

```json
{
  "success": true,
  "data": {
    "access_token": "new_access_token",
    "refresh_token": "new_refresh_token",
    "access_token_expires_at": 1700000000,
    "refresh_token_expires_at": 1701296000
  }
}
```

当前刷新策略：

- 原 Refresh Token 会立即进入黑名单，后续再次使用会返回 `token_revoked`
- 新 Refresh Token 作为新的会话续期凭证
- 当前实现不会在 refresh 时批量吊销旧 Access Token，旧 Access Token 仍可使用到自然过期

失败规则：

- 缺少 `X-Refresh-Token` 时返回 `missing_refresh_token`
- token 非法时返回 `invalid_refresh_token`
- token 类型不匹配时返回 `invalid_token_kind`
- `User-Agent` 不合法或不匹配时返回 `missing_user_agent` / `user_agent_mismatch`
- token 已过期时返回 `token_expired`
- token 已吊销时返回 `token_revoked`
- 黑名单后端不可用时返回 `blacklist_unavailable`
- 新 token 签发失败时返回 `token_refresh_failed`

## 15. 登出接口约定

### 15.1 `POST /auth/logout`

语义：

- 使用当前 Access Token 与当前 Refresh Token 执行显式登出
- 当前实现按“提交到接口的这对 token”进行显式吊销

请求头：

```http
Authorization: Bearer <access_token>
X-Refresh-Token: <refresh_token>
User-Agent: <user-agent>
```

成功响应结构：

```json
{
  "success": true,
  "data": {
    "subject": "user_42",
    "access_token_revoked": true,
    "refresh_token_revoked": true
  }
}
```

当前登出策略：

- 接口会立即吊销当前提交的 Access Token 与 Refresh Token
- 吊销后，这两个 token 再访问受保护接口或刷新接口都会返回 `token_revoked`
- 当前实现不做“同一用户所有历史 token 全量回收”

失败规则：

- 缺少 `Authorization` 时返回 `missing_bearer_token`
- 缺少 `X-Refresh-Token` 时返回 `missing_refresh_token`
- token 非法时返回 `invalid_access_token` / `invalid_refresh_token`
- token 类型不匹配时返回 `invalid_token_kind`
- `User-Agent` 不合法或不匹配时返回 `missing_user_agent` / `user_agent_mismatch`
- token 已过期时返回 `token_expired`
- token 已吊销时返回 `token_revoked`
- Access Token 与 Refresh Token 主体不一致时返回 `token_subject_mismatch`
- 黑名单后端不可用时返回 `blacklist_unavailable`

## 16. 后续实现约束

后续新增接口时必须遵守以下规则：

- Handler 不直接手写新的外层成功响应格式，统一使用现有成功 envelope
- Handler 不直接返回零散字符串错误，统一走错误模型
- 任何错误响应都必须带稳定错误码
- 任何响应都不能缺少 `X-Trace-Id`
- 如果新增了新的基础错误码或新的通用响应字段，必须先更新本文档

## 17. 与代码实现的对应关系

当前文档对应到的主要实现文件如下：

- 统一成功与错误 envelope：`src/interfaces/response.rs`
- `X-Trace-Id` 中间件：`src/interfaces/middleware/trace_id.rs`
- Access Token / Refresh Token 鉴权中间件：`src/interfaces/middleware/authentication.rs`
- 健康检查实现：`src/interfaces/controller/health.rs`
- 认证相关接口：`src/interfaces/controller/auth.rs`
- Google OAuth state 基础设施：`src/infrastructure/auth/google_state.rs`
- 路由装配与 fallback：`src/interfaces/mod.rs`

如果实现发生变化，应先判断：

- 是代码偏离了约定，需要修代码
- 还是契约需要升级，需要先更新本文档

## 18. 维护规则

- 新增通用错误码时，必须同步更新“当前错误码清单”
- 新增统一响应字段时，必须同步更新本文档示例
- 如果未来引入分页结构、列表 envelope、批量错误结构，也必须先在本文档中明确后再实现
