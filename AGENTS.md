# Kiro Repository Agent Guide

Before performing any implementation, refactor, review, documentation, or other execution work in this repository, the agent must use `$rust-best-practices` at `/Users/toothless/.codex/skills/rust-best-practices/SKILL.md`.

## API Design Rules

All HTTP interfaces in this repository must be designed in a strict REST style.

- Use plural resource nouns for collection routes, such as `/users` and `/products`.
- Use resource identifiers in path segments for single-resource routes, such as `/users/{user_id}` and `/products/{product_id}`.
- Do not use verbs in route paths for business actions. Prefer HTTP methods to express intent.
- Follow standard REST method semantics:
  - `GET /resources` for list queries
  - `GET /resources/{id}` for resource details
  - `POST /resources` for creation
  - `PUT /resources/{id}` for full replacement
  - `PATCH /resources/{id}` for partial updates
  - `DELETE /resources/{id}` for deletion
- Use nested resources only when there is a real parent-child relationship, for example `/users/{user_id}/orders`.
- Use query parameters for filtering, sorting, and pagination instead of encoding those concerns into path names.
- Return structured JSON responses and align HTTP status codes with REST semantics.
- Operational endpoints such as `/health`, `/health/live`, and `/health/ready` are allowed as infrastructure exceptions and should not be modeled as business resources.
