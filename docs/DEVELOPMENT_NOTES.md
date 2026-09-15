# Development Notes

This file contains important context and development notes for the Actualised.ai ecosystem to help agents in future sessions. Please consult these notes before making architectural changes.

## 1. N-API TypeScript Definitions
The N-API TypeScript bindings (`native.d.ts`) are **manually maintained**. 
When adding a new native Rust method via the `#[napi]` macro in `packages/sdk/src/lib.rs` (e.g., `pub async fn sync_issue()`), you **MUST** manually add the corresponding TypeScript method signature to `packages/sdk/native.d.ts`. If you fail to do this, the TypeScript compiler (`tsc`) will fail the build process during CI/CD.

## 2. SurrealDB Permissions & Schema Initialization
- `actualised-core` manages its state in SurrealDB using a root connection when running in administrative environments, but drops down to user-level scope via JWT authentication (`db.authenticate()`) for regular sessions.
- **Critical:** Schema definitions (`DEFINE TABLE`, `DEFINE ACCESS`) are strictly administrative operations. `CompanyState::init()` MUST NOT attempt to execute `DEFINE` statements when authenticated via a standard user token, as this will trigger an `IAM error: Not enough permissions to perform this action`. Always wrap schema migrations in an `if token.is_none()` guard.

## 3. Webhook Authentication
Webhooks (e.g., `/api/webhooks/telegram` or `/api/webhooks/github`) typically receive requests from external servers that lack browser cookies (`session_token`) and custom headers (`X-Company-ID`). 
- **Session Tokens:** External webhooks MUST be registered with a query parameter `?token=YOUR_SESSION_TOKEN` so `requireClient()` can successfully authenticate the headless payload against SurrealDB.
- **Company Resolution:** If `X-Company-ID` is not provided, the SDK falls back to checking the `?companyId=` query parameter. If still absent, it will automatically query the database and fallback to the user's first available company.
- **Mock Inference Engine Fallback:** If the `InferenceEngine` is not explicitly configured via the UI, it natively falls back to grabbing the `GEMINI_API_KEY` from the environment variables (e.g., Cloud Run secrets). 

## 4. Terraform & Cloud Run Secrets
When mapping new secrets to the Cloud Run container:
1. They must be registered in the GitHub Repository Secrets.
2. They must be mapped in `.github/workflows/cloud-run-deploy.yml` as `TF_VAR_your_secret_name: ${{ secrets.YOUR_SECRET_NAME }}`.
3. They must be explicitly declared in `infra/variables.tf`.
4. They must be injected into the `env` block of the `google_cloud_run_v2_service` resource in `infra/main.tf`.

## 5. SurrealDB v2 Deprecations & Edge Cases
- **Deprecated `type::thing`**: In SurrealDB v2, `type::thing` is deprecated and will cause a `Parse error: Invalid function/constant path` if used in queries (e.g. `UPDATE type::thing('company', $id)`). You MUST use `type::record` instead (e.g., `UPDATE type::record('company', $id)`).
- **Null / NONE Hydration Crash**: Be careful when hydrating state queries. If a field might be missing (e.g., `parent_id` on root agents), SurrealDB v2 returns `NONE` instead of `NULL`. Using `record::id(parent_id)` will crash the query (`Argument 1 was the wrong type. Expected 'record' but found 'NONE'`). Always wrap it in a strict type check conditional: `IF type::is::record(parent_id) THEN record::id(parent_id) ELSE null END`.

## 6. CSS Cascading & Media Queries
When adding dark mode support in CSS (like `App.css`), place the `@media (prefers-color-scheme: dark)` block at the **bottom** of the file (or after the base class styles it modifies). If the base class is declared *after* the media query, CSS's top-to-bottom cascading rules will cause the base styles to incorrectly override the dark mode styles.
