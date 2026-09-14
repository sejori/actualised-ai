# Actualised.ai — Development & Setup Guide

This guide covers the out-of-the-box company setup flow, webhook integrations, and known SurrealDB / Cloud Run gotchas worth being aware of during development.

---

## 1. Setting up a Company from Scratch (Default Flow)

When a user sets up a new company via the Web UI (or calls `ActualisedClient.foundCompany()`), the engine automatically executes `crates/core/src/default_company.rs`.

This default script seeds a complete out-of-the-box agent hierarchy based on the **"Pawsome"** pet food e-commerce template:

| Agent ID | Role |
|---|---|
| `node_project_manager` | Project Manager (Root Agent) |
| `node_eng_lead` | Engineering Lead |
| `node_eng_frontend` | Frontend Engineer |
| `node_eng_backend` | Backend Engineer |
| `node_product_lead` | Product Lead |
| `node_product_designer` | Product Designer |
| `node_growth_lead` | Growth Lead |
| `node_growth_exec` | Marketing Exec |

This is the documented example used in the Web UI demo and in all code samples throughout this guide.

---

## 2. Custom Agent Networks

You can bypass the default Pawsome seeding and create a fully custom agent hierarchy using the SDK directly. See `packages/sdk/examples/self_developing_company.ts` for a reference implementation.

### Webhook Routing

The Telegram webhook in `server.ts` **no longer hardcodes** a specific agent ID. It dynamically finds the root agent (the one with `parent_id = null`) at runtime:

```typescript
const agents = await client.getAgents();
const rootAgent = agents.find((a: any) => !a.parent_id);
await client.queueMessage(rootAgent.id, instruction);
```

This means any custom agent hierarchy works automatically — no manual server changes required. The root agent (the one at the top of the tree) will always receive incoming Telegram messages.

---

## 3. SurrealDB Gotchas

### 3.1 Silent `CREATE` Permission Failures

> **Critical.** This is the single most dangerous failure mode in the system.

If a SurrealDB table is defined with permissions for `select, update, delete` but **omits `create`**, any `CREATE` query executed via a scoped user token will **silently succeed** (returning `[]`) instead of throwing an error.

**Symptoms:** Records appear to be created but are never actually saved. All subsequent reads return empty arrays, cascading into confusing downstream errors like `Agent not found`.

**Fix:** Always include `create` in table permissions:
```sql
DEFINE TABLE OVERWRITE agent SCHEMALESS PERMISSIONS FOR select, create, update, delete WHERE company_id.owner = $auth.id OR company_id = null;
```

### 3.2 `parent_id` Stored as RecordId, Read Back as String

When an `Agent` struct is written to SurrealDB with `parent_id = "agent:some_id"` (a string), SurrealDB **automatically promotes** it to a typed `RecordId` in storage. On the way back out, a bare `SELECT parent_id` returns the `RecordId` object — which causes a Rust deserialization error:

```
Failed to deserialize field 'parent_id' on type 'Agent': Expected string, got record
```

**Fix:** Always use `record::id(parent_id) AS parent_id` in SELECT queries:
```sql
SELECT record::id(id) AS id, name, role, record::id(parent_id) AS parent_id, ... FROM agent;
```

This applies to any field that stores a link to another record (`company_id`, `parent_id`, `assignee`, etc.).

### 3.3 `companyId` Must Include Table Prefix

When passing a `companyId` as a query parameter or binding it into a SurrealQL `type::record($target)` call, it must include the table prefix (e.g. `company:pkx6k4z9eemy5j17ojun`). A bare ID like `pkx6k4z9eemy5j17ojun` will fail:

```
Could not cast into `record` using input 'pkx6k4z9eemy5j17ojun'
```

**Fix (applied in `state.rs`):** Normalise before binding:
```rust
let full_target = if target.contains(':') {
    target.clone()
} else {
    format!("company:{}", target)
};
```

### 3.4 `DEFINE TABLE IF NOT EXISTS` Won't Update Permissions

If you change table permissions in code (e.g. adding `create`) and the table already exists in the database, `DEFINE TABLE IF NOT EXISTS` is a no-op — the existing definition is preserved.

**Fix:** Use `DEFINE TABLE OVERWRITE` to force-apply the new definition on every app boot:
```sql
DEFINE TABLE OVERWRITE agent SCHEMALESS PERMISSIONS FOR select, create, update, delete WHERE ...;
DEFINE ACCESS OVERWRITE user ON DATABASE TYPE RECORD ...;
```

This is safe to run on every boot; it only changes the schema definition, not any existing data.

### 3.5 `RecordId` Double-Prefix Bug in Rust

When constructing a SurrealDB `RecordId` in Rust, do **not** include the table prefix in the key argument:

```rust
// WRONG — results in agent:⟨agent:some_id⟩
RecordId::new("agent", "agent:some_id")

// CORRECT
RecordId::new("agent", "some_id")
RecordId::new("agent", id.trim_start_matches("agent:"))
```

### 3.6 Agent Bootstrapping and `company_id = null`

When `foundCompany()` is called, the SDK's `seed_default_company()` runs first and creates all agents with `company_id = null` (because the company does not exist in the DB yet). The company record is created afterwards in `set_company_name`.

`set_company_name` includes a critical follow-up update:
```sql
UPDATE agent SET company_id = $c[0].id WHERE company_id = null;
```

This re-links all detached agents to the newly created company. If this step is missing, every subsequent `getCompanies()` call will return an empty array and the system will appear to have no agents or company.

---

## 4. Webhook Authentication

All webhooks (`/api/webhooks/*`) require a valid SurrealDB session token to authenticate the user and retrieve the correct company context. Since external services (Telegram, GitHub) do not send browser cookies, the token must be passed as a query parameter when registering the webhook.

### Registering the Telegram Webhook

```bash
# Step 1: Sign in to get a fresh token
TOKEN=$(curl -s -X POST https://<SURREALDB_URL>/signin \
  -H "Content-Type: application/json" \
  -d '{"NS":"actualised","DB":"core","AC":"user","email":"you@example.com","pass":"yourpassword"}' \
  | jq -r .token)

# Step 2: Register the webhook URL with your token
curl "https://api.telegram.org/bot<BOT_TOKEN>/setWebhook" \
  -d "url=https://<CLOUD_RUN_URL>/api/webhooks/telegram?token=$TOKEN"

# Step 3: Verify
curl "https://api.telegram.org/bot<BOT_TOKEN>/getWebhookInfo"
```

### Token Expiry & Re-registration

> **Important.** Session tokens expire after **30 days**. When a token expires, all webhook calls will return `500 There was a problem with authentication`.

You must re-sign-in and re-register the webhook URL with the new token before it expires. Running `DEFINE ACCESS OVERWRITE` on an existing namespace **also invalidates all currently active session tokens**, requiring immediate re-registration.

**Symptoms of an expired/invalidated token in Cloud Run logs:**
```
POST 500 /api/webhooks/telegram?token=...
Request failed: There was a problem with authentication
```

---

## 5. Cloud Run & Terraform Deployment

To ensure your production Cloud Run environment successfully connects to SurrealDB, Telegram, and the Gemini API, provide the following secrets to your CI/CD pipeline:

1. Add these to **GitHub Repository Secrets**:
   - `SURREALDB_URL`
   - `SURREALDB_USER`
   - `SURREALDB_PASS`
   - `TELEGRAM_TOKEN`
   - `GEMINI_API_KEY`
   - `WIF_PROVIDER` (Workload Identity Federation provider ID)
   - `WSA_EMAIL` (Service Account Email)

2. The GitHub Action (`.github/workflows/cloud-run-deploy.yml`) maps these to `TF_VAR_...` environment variables.

3. Terraform (`infra/main.tf`) injects them directly into the Cloud Run container environment.

### Schema Update on Boot

On every boot, `server.ts` calls `ActualisedClient.create()` using root DB credentials (`SURREALDB_USER` / `SURREALDB_PASS`). This triggers `CompanyState::init()` without a user token, which runs all `DEFINE TABLE OVERWRITE` schema definitions as the database root. This ensures permissions are always up to date after a code change, without requiring a manual migration step.

---

## 6. CI Test Suite

The Rust test suite lives in `crates/core/src/state.rs` (unit tests) and `crates/core/tests/` (integration tests).

### Agent ID Prefix Convention

All agent IDs are normalised to carry the `agent:` prefix throughout the system. Test helpers must enforce this:

```rust
fn create_agent(id: &str, parent_id: Option<&str>) -> Agent {
    let full_id = if id.starts_with("agent:") {
        id.to_string()
    } else {
        format!("agent:{}", id)
    };
    let full_parent = parent_id.map(|p| {
        if p.starts_with("agent:") { p.to_string() } else { format!("agent:{}", p) }
    });
    Agent { id: full_id, parent_id: full_parent, ..Default::default() }
}
```

When asserting on agent IDs returned from `state.agents`, always expect the full prefixed form:
```rust
// WRONG
state.agents.iter().find(|a| a.id == "agent1")
// CORRECT
state.agents.iter().find(|a| a.id == "agent:agent1")
```

---

## 7. Self-Developing Company (Internal Setup)

The owner/developer account uses a custom agent hierarchy instead of the default Pawsome template. This is seeded directly into SurrealDB using the HTTP REST API rather than `foundCompany()`, to avoid triggering the default agent seed.

The team structure is:
```
Product Manager (root)
└── Engineering Manager
    ├── Core Rust Maintainer
    ├── Web UX Developer
    └── SDK Engineer
```

See `packages/sdk/examples/self_developing_company.ts` for the reference seeding script.

To re-seed (e.g. after clearing the database), use the SurrealDB HTTP API directly with a valid user token. Use `CREATE agent CONTENT { ... }` with auto-generated IDs (no explicit `id:` field) and chained `LET $pm = ...` variables to establish parent-child relationships in a single transaction.
