# Actualised.ai Company Setup Guide

This guide covers the out-of-the-box company setup flow, webhook integrations, and how to correctly define custom agent networks without breaking integrations.

## 1. Setting up a Company from Scratch (Default Flow)

When a user sets up a new company via the Web UI (or calls `ActualisedClient.foundCompany()`), the engine automatically executes `crates/core/src/default_company.rs`.

This default script seeds a complete out-of-the-box agent hierarchy based on the "Pawsome" template, which includes:
- `node_project_manager` (Project Manager / Root Agent)
- `node_eng_lead` (Engineering Lead)
- `node_eng_frontend` (Frontend Engineer)
- `node_eng_backend` (Backend Engineer)
- `node_product_lead` (Product Lead)
- `node_product_designer` (Product Designer)
- `node_growth_lead` (Growth Lead)
- `node_growth_exec` (Marketing Exec)

Because this is the default out-of-the-box structure, **the Telegram webhook in `server.ts` is explicitly hardcoded to route all incoming messages directly to `node_project_manager`**.

## 2. Custom Agent Networks & Webhook Routing

If you choose to bypass the default company seeding and create a fully custom agent network (e.g., via a script like `self_developing_company.ts`), you must ensure your webhook routing matches your custom Agent IDs.

If your script creates a root agent named `pm`, but `packages/sdk/examples/server.ts` is still routing messages to `node_project_manager`, the webhook will return a `200 OK` (authenticating successfully) but silently fail in the background with an `[Error: Agent not found]`.

**To fix this when building custom networks:**
1. Either assign your custom root agent the ID `node_project_manager`.
2. OR update `packages/sdk/examples/server.ts` to route to your custom agent's ID:
   ```typescript
   // Update from:
   await client.queueMessage('node_project_manager', instruction);
   // To:
   await client.queueMessage('your_custom_agent_id', instruction);
   ```

## 3. Webhook Authentication

All webhooks (`/api/webhooks/*`) require a valid SurrealDB session token to authenticate the user and retrieve the correct company context. Since external services (like Telegram or GitHub) do not send browser cookies, you must pass the token as a query parameter when registering the webhook.

**Example Telegram Registration:**
```bash
curl "https://api.telegram.org/bot<BOT_TOKEN>/setWebhook?url=https://<YOUR_CLOUD_RUN_URL>/api/webhooks/telegram?token=<YOUR_SESSION_TOKEN>"
```
If the token is invalid or missing, the webhook will crash with a `500 InvalidToken` error.

## 4. Cloud Run & Terraform Deployments

To ensure your production Cloud Run environment successfully connects to SurrealDB and the Gemini API, you must provide the necessary secrets to your CI/CD pipeline:
1. Add `GEMINI_API_KEY`, `SURREALDB_URL`, `SURREALDB_USER`, and `SURREALDB_PASS` to your GitHub Repository Secrets.
2. The GitHub Action (`.github/workflows/cloud-run-deploy.yml`) will map these to `TF_VAR_...` variables.
3. Terraform (`infra/main.tf`) injects these directly into the Cloud Run container environment.
