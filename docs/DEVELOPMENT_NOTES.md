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
