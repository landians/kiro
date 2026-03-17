# Kiro Repository Agent Guide

This file defines the default execution rules for any agent working in this repository. Treat it as the repository-level operating contract. Unless the user explicitly asks for an exception, all implementation, refactor, and documentation work must follow these rules.

## 1. Core Principles

- Follow DDD-style layered boundaries inspired by `food-app-server`, but expressed in this Rust codebase.
- Keep dependency direction stable: `interfaces -> application -> domain`, while `infrastructure` only provides implementations and technical assembly support.
- Use `main.rs` and bootstrap code as the composition root. Do not move business logic into the entrypoint.
- Prefer clarity, maintainability, and testability over cleverness or fewer lines of code.
- Favor readable public APIs and strong type expression over shaving off a few lines of code.

## 2. Rust Coding Rules

- Use Rust 2024 Edition conventions.
- Do not introduce `unsafe` unless there is an explicit ADR-approved exception.
- Runtime APIs use native `async fn`.
- Async traits should prefer `fn -> impl Future`; do not introduce `async_trait` unless there is a reviewed need, and when implementing those trait methods, use `async fn` in the impl.
- Prefer static dispatch over `dyn`-based runtime polymorphism unless plugin-style runtime selection is actually required.
- Domain errors must use `thiserror` with explicit semantics.
- `anyhow` is allowed only for application/bootstrap/tooling error aggregation, not as the long-term domain error model.
- Error types should retain enough structure for logging, HTTP responses, alerting, and debugging.
- Preserve original error semantics when crossing layers; do not collapse everything into context-free strings in the middle of the call chain.

## 3. Code Flattening Rules

- Keep the main path linearly readable; push failure paths to the top of the function with guard clauses.
- Prefer early `return`, `continue`, and `break` over nested `if/else` pyramids.
- Prefer `?`, `let-else`, and small helper functions over deeply nested `match` or `if let` control flow.
- When a function mixes validation, loading, transformation, persistence, and observability, keep the top-level function as step orchestration only and move details into clearly named private helpers.
- Name complex intermediate values before calling downstream functions; avoid deeply nested constructor calls inside a single expression.
- If an API or repository exposes batch semantics such as `batch_*` or `m*`, do not silently degrade it into N single remote calls unless the backend truly lacks batch capability and the code comments explain the exception.
- Keep logs, metrics, and trace enrichment concentrated at clear entry/exit points instead of scattering observability code through every branch.
- Do not over-flatten by splitting code into many trivial 1-2 line functions or by building unreadable long `map/and_then` chains; readability and correctness remain the primary goal.
- For tests, keep `arrange -> act -> assert` structure, keep one test focused on one behavior, and extract repetitive setup into helpers.
- Prefer a refactor flow of `validate -> load -> transform -> persist -> observe`, and verify behavior after structural changes, especially on error and edge paths.
- For performance-sensitive paths, benchmark after flattening-oriented refactors.

## 4. Layer Responsibilities

### 4.1 `interfaces/`

Responsibilities:

- HTTP routing
- request parsing and validation
- response shaping
- middleware
- auth context extraction

Do not place in `interfaces/`:

- business rules
- direct database access
- direct Redis access
- direct third-party SDK usage

Rule of thumb:

- if a piece of logic still makes sense without HTTP, it should not live in `interfaces/`

### 4.2 `interfaces/controller` Boundary

Controllers may depend on:

- `crate::application::*` services or use-case entrypoints
- `crate::interfaces::dto::*`
- `crate::interfaces::response::*`
- `crate::interfaces::AppState`
- framework request/response extraction types
- request-scoped context such as trace or auth claims

Controllers must not depend on:

- `crate::infrastructure::persistence::*`
- `sqlx::*`, `redis::*`, raw database pools, raw Redis clients
- repository implementation types
- direct third-party SDK clients

Controller rule:

- translate HTTP input into application input, call application, translate output into HTTP response

### 4.3 `interfaces/middleware` Boundary

Middleware may depend on:

- trace/auth/rate-limit/audit related lightweight services
- already assembled application services from `AppState`
- claims/context extension types

Middleware must not depend on:

- raw `PgPool`, raw Redis connections, third-party SDK clients
- repository implementations
- complex business orchestration

Middleware rule:

- only handle cross-cutting concerns before or after controllers, such as `X-Trace-Id`, token verification, and admin identity injection

### 4.4 `application/`

Responsibilities:

- orchestrate use cases
- define transaction boundaries
- coordinate domain objects and repository traits
- coordinate sync calls and async event publishing
- expose application services by module

Do not place in `application/`:

- direct HTTP types such as `HeaderMap` or axum extractors
- direct SQL building
- direct third-party SDK implementation details
- large concrete service implementations inside `application/mod.rs`
- hard-coding domain rules as ad hoc workflow scripts

Application rule:

- application decides how a use case is completed, but not what the core business rule fundamentally is
- keep each module service in its own file or subdirectory, and keep startup-only assembly logic in the composition root rather than disguising it as an application service
- `interfaces/` may depend on application services only; do not bypass application and read infrastructure objects directly

### 4.5 `domain/`

Responsibilities:

- entities
- value objects
- domain services
- repository traits
- domain errors

Domain must not depend on:

- `axum`
- database drivers
- Redis clients
- third-party payment/login/mail SDKs

### 4.6 `infrastructure/`

Responsibilities:

- database implementations
- Redis implementations
- third-party adapters
- external clients
- technical bootstrap/builders

Infrastructure rules:

- implement abstractions defined by `domain` or consumed by `application`
- absorb external system differences and avoid leaking SDK details upward
- prefer builder-style construction for connection pools, clients, adapters, and configurable technical services
- do not create empty wrapper types such as `AppInfrastructure` or `AuthInfrastructure` just to re-bundle dependencies
- expose concrete technical building blocks directly when possible, such as `PgPool`, `JwtService`, `TokenBlacklistService`, or repository implementations
- if bootstrap returns a resource set, it must represent initialized technical resources rather than acting as a disguised application-service container
- names such as repository, client, adapter, and builder should reflect real technical responsibility; avoid vague names like `manager` or generic `infrastructure`

## 5. Application Naming and File Template

Naming rules:

- prefer business-module names such as `application/auth.rs`, `application/admin.rs`, `application/health.rs`
- when a module has one main application service, name it `XxxService`
- when a module grows, split it into a directory such as:
  - `application/admin/mod.rs`
  - `application/admin/commands.rs`
  - `application/admin/queries.rs`
  - `application/admin/service.rs`
- keep `application/mod.rs` limited to exports, grouping, and the service container
- name methods by business intent, such as `create_admin`, `list_admin_users`, `revoke_session_tokens`

Default application template:

```rust
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
        todo!()
    }
}
```

Template constraints:

- store only dependencies needed for that use case module
- inject dependencies in `new(...)`; do not perform IO or migrations in constructors
- methods orchestrate use cases, transactions, and coordination
- application may depend on repository traits, domain services, event publishers, and auth services
- application must not depend on concrete persistence implementation types when a trait boundary is expected

## 6. Repository Layout Standard

Preferred layout:

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

Repository rules:

- define repository traits under `domain/repository/`
- place concrete implementations under `infrastructure/persistence/**`
- PostgreSQL implementations use names like `PostgresAdminRepository`
- Redis implementations use names like `RedisTokenBlacklistRepository`
- file names stay in snake_case and align with the main type
- repository constructors only take technical dependencies and necessary config
- repository implementations must not leak raw driver rows or low-level driver errors into controllers
- if a backend currently has only a builder or connectivity helper, a single file is acceptable; once multiple repositories appear, split into `postgres/` or `redis/` subdirectories

## 7. Composition Root Rules

- `main.rs` is the only composition root unless a dedicated bootstrap module is explicitly introduced
- composition order should remain:
  - load config
  - initialize infrastructure
  - build repositories/adapters
  - build application services
  - inject router/state
  - start server
- `AppState` should contain configuration, application services, and runtime metadata
- `AppState` must not directly own low-level infrastructure such as raw pools or clients unless there is a narrowly justified cross-cutting technical need

## 8. API and Interface Rules

- APIs follow RESTful style
- route paths are resource-oriented, not action-oriented
- group routes by module
- keep HTTP verb semantics aligned with intent: `GET` for reads, `POST` for create/actions requiring creation semantics, `PUT/PATCH` for updates, `DELETE` for deletion or revocation
- every response must include `X-Trace-Id`
- if the incoming request already carries an acceptable trace id, validate and propagate it; otherwise generate one at the edge
- error responses must still include `X-Trace-Id`
- keep success and error response envelopes stable
- follow `/Volumes/Extensions/code/kiro/docs/project-api-contract-standards.md` for the unified API contract

## 9. Authentication Rules

- protected APIs use JWT Bearer authentication
- use stateless JWT + Refresh Token
- do not persist JWT or refresh token records in relational tables
- access token default lifetime: 2 hours
- refresh token default lifetime: 15 days
- refresh tokens are only for token renewal, not resource access
- business APIs send access tokens via `Authorization: Bearer <jwt>`; refresh flows use a dedicated refresh-token header or an equivalent dedicated secure channel
- JWT and refresh token claims must include `sub`, `jti`, and a User-Agent digest such as `ua_hash`
- authentication middleware must verify signature, expiration, and `ua_hash`
- protected resource APIs must pass blacklist checks
- logout, revocation, and security incidents must revoke both access and refresh tokens via the configured blacklist mechanism
- User-Agent digests are only a supplemental binding signal; they do not replace rate limiting, risk control, or audit logging

## 10. Observability and Security

- every HTTP request must have trace context
- every key business action should log module, action, and result
- failures should be traceable by error category and trace id
- observability should align with OpenTelemetry
- authentication secrets and blacklist behavior must be configuration-driven
- when Redis is shared across environments or services, isolate keys via `REDIS_KEY_PREFIX`
- store normalized User-Agent digests rather than raw oversized User-Agent strings inside tokens when possible
- refresh failures, `ua_hash` mismatches, and refresh abuse should be auditable and rate-limitable
- revocation-policy changes should take effect immediately for subsequent requests; do not rely on stale in-memory blacklist state after configuration changes

## 11. Data and Schema Rules

- PostgreSQL business tables use auto-incrementing primary keys, preferably `bigint generated always as identity`
- foreign keys must stay type-compatible with their target primary keys and should also use `bigint`
- do not mix UUID primary keys and integer primary keys arbitrarily across core business tables
- if a stable external identifier is needed, add a separate business code column instead of overloading the primary key

## 12. Reliability and Idempotency Rules

- flows such as payment callbacks, invitation acceptance, email resend, and message consumption must be idempotent
- idempotency keys may live in PostgreSQL or Redis, but ownership and expiration policy must be explicit

## 13. Testing Rules

- domain layer: pure business unit tests first
- application layer: use-case orchestration and transaction-boundary tests
- interface layer: route and response-contract tests
- infrastructure layer: integration tests for PostgreSQL, Redis, and external adapters
- when tests depend on external systems such as DB/Redis/HTTP services, prefer a clear "skip if unavailable" strategy so local execution remains practical

## 14. Documentation Governance

- update the project plan before adding a new module
- update API contract documentation before changing public interfaces
- prefer ADRs for design exceptions rather than silently drifting in code
- the detailed source documents for these rules are `/Volumes/Extensions/code/kiro/docs/project-development-stanards.md` and `/Volumes/Extensions/code/kiro/docs/code-flattening-guideline.md`; if AGENTS and those documents ever diverge, align them immediately
